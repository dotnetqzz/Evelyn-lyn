#!/usr/bin/env python3
import os
import sys
import glob
import re
import subprocess
import time
from pathlib import Path
from concurrent.futures import ThreadPoolExecutor, as_completed

EXE_SUFFIX = ".exe" if os.name == "nt" else ""
AVELYN_EXE = str(Path(__file__).parent / "avelyn" / "target" / "release" / f"avelyn{EXE_SUFFIX}")
TESTS_DIR = str(Path(__file__).parent / "Tests")
TEMP_DIR = str(Path(__file__).parent / "scratch" / "dual_test_bins")

os.makedirs(TEMP_DIR, exist_ok=True)

# Regex patterns to normalize volatile outputs (timers, memory addresses, random hashes, timestamps)
NORMALIZE_PATTERNS = [
    (r"\d+\.\d{3,} seconds", "<TIME> seconds"),
    (r"Total Execution Time [^\n]+\n[0-9\.\+eE-]+", "Total Execution Time: <TIME>"),
    (r"Total Execution Time [^\n]+", "Total Execution Time: <TIME>"),
    (r"Time:\s*[\d\.]+\s*seconds", "Time: <TIME> seconds"),
    (r"Time taken:\s*[\d\.]+\s*seconds", "Time taken: <TIME> seconds"),
    (r"Throughput:\s*\d+\s*iterations / sec", "Throughput: <SPEED> iterations / sec"),
    (r'"created_at":\s*\d+', '"created_at": <TIMESTAMP>'),
    (r"Verification Token ID:\s*[a-f0-9]+", "Verification Token ID: <HASH>"),
    (r"Generated Transaction Cryptographic Signature:\s*[a-f0-9]+", "Signature: <HASH>"),
    (r"Secure Token Hex:\s*[a-f0-9]+", "Secure Token Hex: <HASH>"),
    (r"0x[0-9a-fA-F]+", "<PTR>"),
]

def normalize_output(text: str) -> str:
    cleaned = text.replace("\r\n", "\n").strip()
    for pattern, replacement in NORMALIZE_PATTERNS:
        cleaned = re.sub(pattern, replacement, cleaned)
    return cleaned

def run_single_test(test_file: str):
    name = Path(test_file).stem
    out_bin = os.path.join(TEMP_DIR, f"{name}{EXE_SUFFIX}")
    
    result = {
        "file": test_file,
        "name": name,
        "interp_ok": False,
        "interp_exit": None,
        "interp_out": "",
        "interp_err": "",
        "interp_time": 0.0,
        "compile_ok": False,
        "compile_exit": None,
        "compile_err": "",
        "native_ok": False,
        "native_exit": None,
        "native_out": "",
        "native_err": "",
        "native_time": 0.0,
        "match": False,
        "diff_reason": "",
        "is_false_pass": False,
        "notes": []
    }

    # 1. Run Interpreter
    t0 = time.perf_counter()
    try:
        proc = subprocess.run(
            [AVELYN_EXE, test_file],
            capture_output=True,
            text=True,
            errors="replace",
            timeout=45
        )
        result["interp_time"] = time.perf_counter() - t0
        result["interp_exit"] = proc.returncode
        result["interp_out"] = proc.stdout
        result["interp_err"] = proc.stderr
        result["interp_ok"] = (proc.returncode == 0)
    except subprocess.TimeoutExpired:
        result["interp_time"] = 45.0
        result["interp_err"] = "TIMEOUT (45s)"
        result["interp_ok"] = False
    except Exception as e:
        result["interp_err"] = str(e)
        result["interp_ok"] = False

    # 2. Compile to Native
    try:
        compile_proc = subprocess.run(
            [AVELYN_EXE, "compile", test_file, "-o", out_bin, "-O2"],
            capture_output=True,
            text=True,
            errors="replace",
            timeout=900
        )
        result["compile_exit"] = compile_proc.returncode
        result["compile_err"] = compile_proc.stderr
        result["compile_ok"] = (compile_proc.returncode == 0 and os.path.exists(out_bin))
    except subprocess.TimeoutExpired:
        result["compile_err"] = "COMPILE TIMEOUT (45s)"
        result["compile_ok"] = False
    except Exception as e:
        result["compile_err"] = str(e)
        result["compile_ok"] = False

    # 3. Run Native Executable
    if result["compile_ok"]:
        t0 = time.perf_counter()
        try:
            native_proc = subprocess.run(
                [out_bin],
                capture_output=True,
                text=True,
                errors="replace",
                timeout=45
            )
            result["native_time"] = time.perf_counter() - t0
            result["native_exit"] = native_proc.returncode
            result["native_out"] = native_proc.stdout
            result["native_err"] = native_proc.stderr
            result["native_ok"] = (native_proc.returncode == 0)
        except subprocess.TimeoutExpired:
            result["native_time"] = 6000.0
            result["native_err"] = "NATIVE TIMEOUT (45s)"
            result["native_ok"] = False
        except Exception as e:
            result["native_err"] = str(e)
            result["native_ok"] = False

    # 4. Compare outputs & detect silent bugs / false passes
    norm_interp = normalize_output(result["interp_out"])
    norm_native = normalize_output(result["native_out"])

    if result["interp_ok"] and result["native_ok"]:
        if norm_interp == norm_native:
            result["match"] = True
        else:
            result["match"] = False
            result["diff_reason"] = "OUTPUT_MISMATCH"
    elif result["interp_ok"] and not result["native_ok"]:
        result["match"] = False
        result["diff_reason"] = "NATIVE_FAILED_BUT_INTERP_PASSED"
    elif not result["interp_ok"] and result["native_ok"]:
        result["match"] = False
        result["diff_reason"] = "INTERP_FAILED_BUT_NATIVE_PASSED"
    else:
        result["match"] = False
        result["diff_reason"] = "BOTH_FAILED"

    # Check false passes (e.g. exit code 0 but empty output or error text)
    if result["interp_ok"] and len(norm_interp) == 0:
        result["notes"].append("EMPTY_INTERP_OUTPUT")
    if result["native_ok"] and len(norm_native) == 0:
        result["notes"].append("EMPTY_NATIVE_OUTPUT")
    if "Error:" in norm_interp and result["interp_exit"] == 0 and "test" not in name:
        result["is_false_pass"] = True
        result["notes"].append("INTERP_ERROR_WITH_EXIT_0")
    if "Error:" in norm_native and result["native_exit"] == 0 and "test" not in name:
        result["is_false_pass"] = True
        result["notes"].append("NATIVE_ERROR_WITH_EXIT_0")

    return result

def main():
    test_files = sorted(glob.glob(os.path.join(TESTS_DIR, "*.lyn")))
    print(f"================================================================================", flush=True)
    print(f" AVELYN DUAL EXECUTION TEST SUITE (INTERPRETER vs COMPILED AIR/LLVM)", flush=True)
    print(f" Total test files: {len(test_files)}", flush=True)
    print(f"================================================================================\n", flush=True)

    results = []
    with ThreadPoolExecutor(max_workers=6) as executor:
        futures = {executor.submit(run_single_test, f): f for f in test_files}
        for future in as_completed(futures):
            res = future.result()
            results.append(res)
            
            status_sym = "MATCH" if res["match"] else "DIFF"
            if not res["compile_ok"]:
                status_sym = "COMPILE_FAIL"
            elif not res["interp_ok"]:
                status_sym = "INTERP_FAIL"

            print(f"[{status_sym:<13}] {res['name']:<35} (Interp: {res['interp_time']:.2f}s, Native: {res['native_time']:.2f}s)", flush=True)
            if not res["match"] and res["diff_reason"]:
                print(f"   -> Reason: {res['diff_reason']}", flush=True)
                if res["compile_err"]:
                    print(f"   -> Compile Err: {res['compile_err'].strip()[:150]}", flush=True)
                if res["native_err"]:
                    print(f"   -> Native Err: {res['native_err'].strip()[:150]}", flush=True)

    results.sort(key=lambda x: x["name"])

    # Summary Statistics
    total = len(results)
    exact_matches = sum(1 for r in results if r["match"])
    interp_passed = sum(1 for r in results if r["interp_ok"])
    compile_passed = sum(1 for r in results if r["compile_ok"])
    native_passed = sum(1 for r in results if r["native_ok"])
    false_passes = sum(1 for r in results if r["is_false_pass"])

    print("\n" + "="*80, flush=True)
    print(" COMPREHENSIVE DUAL TEST SUMMARY", flush=True)
    print("="*80, flush=True)
    print(f" Total Tests Executed:      {total}", flush=True)
    print(f" Interpreter Passed:        {interp_passed}/{total} ({interp_passed/total*100:.1f}%)", flush=True)
    print(f" Native Compiler Succeeded: {compile_passed}/{total} ({compile_passed/total*100:.1f}%)", flush=True)
    print(f" Native Binary Passed:      {native_passed}/{total} ({native_passed/total*100:.1f}%)", flush=True)
    print(f" Exact Output Matches:      {exact_matches}/{total} ({exact_matches/total*100:.1f}%)", flush=True)
    print(f" False Passes Detected:     {false_passes}", flush=True)
    print("="*80, flush=True)

    # Save detailed JSON/text report
    report_file = os.path.join(os.path.dirname(__file__), "dual_test_report.txt")
    with open(report_file, "w", encoding="utf-8") as f:
        f.write(f"AVELYN DUAL TEST REPORT\n")
        f.write(f"Total: {total}, Exact Matches: {exact_matches}, Interp Passed: {interp_passed}, Native Passed: {native_passed}\n\n")
        for r in results:
            f.write(f"[{r['name']}]\n")
            f.write(f"  Match: {r['match']} ({r['diff_reason']})\n")
            f.write(f"  Interp OK: {r['interp_ok']} (Exit: {r['interp_exit']}, Time: {r['interp_time']:.3f}s)\n")
            f.write(f"  Compile OK: {r['compile_ok']} (Exit: {r['compile_exit']})\n")
            f.write(f"  Native OK: {r['native_ok']} (Exit: {r['native_exit']}, Time: {r['native_time']:.3f}s)\n")
            if r['notes']:
                f.write(f"  Notes: {', '.join(r['notes'])}\n")
            if not r['match']:
                f.write(f"  Interp Out (first 300):\n{r['interp_out'][:300]}\n")
                f.write(f"  Native Out (first 300):\n{r['native_out'][:300]}\n")
                if r['compile_err']:
                    f.write(f"  Compile Err:\n{r['compile_err'][:300]}\n")
                if r['native_err']:
                    f.write(f"  Native Err:\n{r['native_err'][:300]}\n")
            f.write("\n")
    print(f"\nDetailed report written to: {report_file}", flush=True)

if __name__ == "__main__":
    main()
