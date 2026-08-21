#!/usr/bin/env python3
"""
run_all_tests.py — Single Global Unified Test Runner for Evelyn (Avelyn).

Features:
- Executes every single .lyn test in Tests/ line by line with real-time status output.
- Execution Modes:
    * `interpreter` : Execute all tests in interpreter mode.
    * `native` / `compiler` : Compile tests to native binaries via LLVM (-O2) and execute.
    * `dual` / `both` : Run both modes, normalize volatile outputs, and verify parity.
- 5-Minute (300s) Timeout per test across all compilation and execution stages.
- Sequential line-by-line execution or multi-threaded parallel execution.
- Pattern filtering (--filter pattern).
- Category-wise reporting and detailed failure diagnostics.

Usage:
  python run_all_tests.py                            # Run all tests sequentially line by line
  python run_all_tests.py --mode native              # Run all tests with native LLVM compiler
  python run_all_tests.py --mode dual                # Dual-mode (runs both & checks parity)
  python run_all_tests.py --jobs 4                   # Run with 4 parallel worker threads
  python run_all_tests.py --filter algo              # Run only tests matching 'algo'
  python run_all_tests.py --fail-fast                # Stop on first failure
"""

import argparse
import glob
import os
import re
import shutil
import subprocess
import sys
import threading
import time
from concurrent.futures import ThreadPoolExecutor, as_completed
from pathlib import Path

# Ensure UTF-8 output formatting on Windows
if sys.platform == "win32":
    try:
        sys.stdout.reconfigure(encoding='utf-8', errors='replace')
        sys.stderr.reconfigure(encoding='utf-8', errors='replace')
    except Exception:
        pass

EXE_SUFFIX = ".exe" if os.name == "nt" else ""
TEMP_BINS_DIR = os.path.abspath("scratch/test_bins")
os.makedirs(TEMP_BINS_DIR, exist_ok=True)
print_lock = threading.Lock()

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

def find_avelyn_bin(provided_path=None):
    if provided_path and os.path.exists(provided_path):
        return os.path.abspath(provided_path)

    candidates = [
        os.path.abspath("avelyn/target/release/Avelyn.exe"),
        os.path.abspath("avelyn/target/release/Avelyn"),
        os.path.abspath("avelyn/target/release/avelyn.exe"),
        os.path.abspath("avelyn/target/release/avelyn"),
        os.path.abspath("avelyn/target/debug/Avelyn.exe"),
        os.path.abspath("avelyn/target/debug/Avelyn"),
        os.path.abspath("avelyn/target/debug/avelyn.exe"),
        os.path.abspath("avelyn/target/debug/avelyn"),
        "Avelyn.exe",
        "Avelyn",
        "avelyn.exe",
        "avelyn"
    ]

    for cand in candidates:
        if os.path.exists(cand):
            return cand
    return "Avelyn"

def run_single_test_interpreter(avelyn_bin, test_file, timeout=300):
    start = time.perf_counter()
    try:
        proc = subprocess.run(
            [avelyn_bin, test_file],
            capture_output=True,
            text=True,
            encoding="utf-8",
            errors="replace",
            timeout=timeout
        )
        dur = time.perf_counter() - start
        return {
            "ok": proc.returncode == 0,
            "exit": proc.returncode,
            "stdout": proc.stdout,
            "stderr": proc.stderr,
            "time": dur,
            "error_type": "" if proc.returncode == 0 else "FAIL"
        }
    except subprocess.TimeoutExpired:
        dur = time.perf_counter() - start
        return {
            "ok": False,
            "exit": -1,
            "stdout": "",
            "stderr": f"TIMEOUT: Exceeded 5 minutes ({timeout}s) limit",
            "time": dur,
            "error_type": "TIMEOUT"
        }
    except Exception as e:
        dur = time.perf_counter() - start
        return {
            "ok": False,
            "exit": -1,
            "stdout": "",
            "stderr": str(e),
            "time": dur,
            "error_type": "EXCEPTION"
        }

def run_single_test_native(avelyn_bin, test_file, temp_dir, timeout=300):
    stem = Path(test_file).stem
    nonce = int(time.perf_counter() * 1000000)
    out_bin = os.path.join(temp_dir, f"{stem}_{nonce}{EXE_SUFFIX}")
    start = time.perf_counter()

    # 1. Compile via LLVM
    try:
        c_proc = subprocess.run(
            [avelyn_bin, "compile", test_file, "-o", out_bin, "-O2"],
            capture_output=True,
            text=True,
            encoding="utf-8",
            errors="replace",
            timeout=timeout
        )
        compile_time = time.perf_counter() - start
        if c_proc.returncode != 0 or not os.path.exists(out_bin):
            return {
                "ok": False,
                "compile_ok": False,
                "exit": c_proc.returncode,
                "stdout": c_proc.stdout,
                "stderr": f"Compilation Failed (code {c_proc.returncode}):\n{c_proc.stderr.strip()}",
                "time": compile_time,
                "error_type": "COMPILE_FAIL"
            }
    except subprocess.TimeoutExpired:
        return {
            "ok": False,
            "compile_ok": False,
            "exit": -1,
            "stdout": "",
            "stderr": f"COMPILE TIMEOUT: Exceeded 5 minutes ({timeout}s) limit",
            "time": time.perf_counter() - start,
            "error_type": "COMPILE_TIMEOUT"
        }
    except Exception as e:
        return {
            "ok": False,
            "compile_ok": False,
            "exit": -1,
            "stdout": "",
            "stderr": f"Compilation Exception: {e}",
            "time": time.perf_counter() - start,
            "error_type": "COMPILE_EXCEPTION"
        }

    # 2. Execute native binary
    t_run = time.perf_counter()
    try:
        r_proc = subprocess.run(
            [out_bin],
            capture_output=True,
            text=True,
            encoding="utf-8",
            errors="replace",
            timeout=timeout
        )
        run_time = time.perf_counter() - t_run
        tot_time = time.perf_counter() - start
        return {
            "ok": r_proc.returncode == 0,
            "compile_ok": True,
            "exit": r_proc.returncode,
            "stdout": r_proc.stdout,
            "stderr": r_proc.stderr,
            "time": tot_time,
            "run_time": run_time,
            "error_type": "" if r_proc.returncode == 0 else "RUN_FAIL"
        }
    except subprocess.TimeoutExpired:
        return {
            "ok": False,
            "compile_ok": True,
            "exit": -1,
            "stdout": "",
            "stderr": f"NATIVE EXEC TIMEOUT: Exceeded 5 minutes ({timeout}s) limit",
            "time": time.perf_counter() - start,
            "error_type": "EXEC_TIMEOUT"
        }
    except Exception as e:
        return {
            "ok": False,
            "compile_ok": True,
            "exit": -1,
            "stdout": "",
            "stderr": f"Execution Exception: {e}",
            "time": time.perf_counter() - start,
            "error_type": "EXEC_EXCEPTION"
        }
    finally:
        if os.path.exists(out_bin):
            try:
                os.remove(out_bin)
            except OSError:
                pass

def execute_dual_test(avelyn_bin, test_file, temp_dir, timeout=300):
    name = Path(test_file).stem
    interp_res = run_single_test_interpreter(avelyn_bin, test_file, timeout)
    native_res = run_single_test_native(avelyn_bin, test_file, temp_dir, timeout)

    norm_interp_out = normalize_output(interp_res["stdout"])
    norm_native_out = normalize_output(native_res["stdout"])

    match = False
    diff_reason = ""

    if interp_res["ok"] and native_res["ok"]:
        if norm_interp_out == norm_native_out:
            match = True
        else:
            diff_reason = "STDOUT Output Mismatch"
    elif interp_res["ok"] and not native_res["ok"]:
        diff_reason = f"Native failed ({native_res['error_type']}) while Interpreter passed"
    elif not interp_res["ok"] and native_res["ok"]:
        diff_reason = f"Interpreter failed ({interp_res['error_type']}) while Native passed"
    else:
        diff_reason = f"Both failed (Interp: {interp_res['error_type']}, Native: {native_res['error_type']})"

    return {
        "name": name,
        "path": test_file,
        "interp": interp_res,
        "native": native_res,
        "match": match,
        "diff_reason": diff_reason
    }

def main():
    parser = argparse.ArgumentParser(description="Evelyn (Avelyn) Global Test Runner")
    parser.add_argument("--bin", type=str, help="Path to avelyn binary executable")
    parser.add_argument("--dir", type=str, default="Tests", help="Directory containing .lyn tests")
    parser.add_argument("--mode", choices=["interpreter", "native", "compiler", "dual", "both"], default="interpreter", help="Execution mode (default: interpreter)")
    parser.add_argument("--filter", type=str, default="", help="Filter tests by name or substring")
    parser.add_argument("--jobs", type=int, default=1, help="Parallel worker threads (default: 1 for line-by-line sequential execution)")
    parser.add_argument("--timeout", type=int, default=300, help="Per-test timeout in seconds (default: 300s / 5 minutes)")
    parser.add_argument("--fail-fast", action="store_true", help="Stop execution on first failure")
    parser.add_argument("--verbose", action="store_true", help="Print stdout/stderr for each test")
    parser.add_argument("--report", type=str, default="test_report.txt", help="Path to save report summary")
    args = parser.parse_args()

    avelyn_bin = find_avelyn_bin(args.bin)
    if not os.path.exists(avelyn_bin) and avelyn_bin != "avelyn":
        print(f"Error: Avelyn binary not found at '{avelyn_bin}'", file=sys.stderr)
        sys.exit(1)

    mode = "dual" if args.mode in ["dual", "both"] else ("native" if args.mode in ["native", "compiler"] else "interpreter")

    pattern = os.path.join(os.path.abspath(args.dir), "**", "*.lyn")
    all_test_files = sorted(glob.glob(pattern, recursive=True))

    if args.filter:
        all_test_files = [f for f in all_test_files if args.filter.lower() in os.path.basename(f).lower()]

    total_tests = len(all_test_files)
    print("  EVELYN (AVELYN) GLOBAL TEST RUNNER — LINE-BY-LINE EXECUTION")
    print(f"  Binary Executable : {avelyn_bin}")
    print(f"  Execution Mode    : {mode.upper()}")
    print(f"  Total Test Files  : {total_tests}")
    print(f"  Workers / Jobs    : {args.jobs} ({'Sequential Line-by-Line' if args.jobs == 1 else 'Parallel'})")
    print(f"  Per-Test Timeout  : {args.timeout}s (5 minutes limit)\n")

    if not all_test_files:
        print("No test files found matching criteria.")
        return

    t_start = time.time()
    passed_count = 0
    failed_count = 0
    failures = []
    completed_counter = 0

    if mode in ["interpreter", "native"]:
        runner_fn = run_single_test_interpreter if mode == "interpreter" else lambda b, f, t: run_single_test_native(b, f, TEMP_BINS_DIR, t)

        if args.jobs == 1:
            for idx, test_file in enumerate(all_test_files, 1):
                test_name = os.path.basename(test_file)
                res = runner_fn(avelyn_bin, test_file, args.timeout)
                
                if res["ok"]:
                    passed_count += 1
                    status_str = "\033[92mPASS\033[0m" if sys.stdout.isatty() else "PASS"
                    print(f"[{idx:4d}/{total_tests:4d}] [{status_str}] {test_name:<48} ({res['time']:.3f}s)")
                else:
                    failed_count += 1
                    status_str = "\033[91mFAIL\033[0m" if sys.stdout.isatty() else "FAIL"
                    err_msg = res["stderr"].strip() or res["stdout"].strip()
                    first_err_line = err_msg.splitlines()[0] if err_msg else "Unknown error"
                    print(f"[{idx:4d}/{total_tests:4d}] [{status_str}] {test_name:<48} ({res['time']:.3f}s) — {first_err_line[:60]}")
                    failures.append((test_name, res["error_type"], err_msg))
                    if args.fail_fast:
                        print("\n[STOPPED] --fail-fast triggered on failure.")
                        break

                if args.verbose and (res["stdout"].strip() or res["stderr"].strip()):
                    if res["stdout"].strip():
                        print("    STDOUT:", res["stdout"].strip())
                    if res["stderr"].strip():
                        print("    STDERR:", res["stderr"].strip())

        else:
            # Parallel execution with thread-safe line-by-line output
            with ThreadPoolExecutor(max_workers=args.jobs) as executor:
                future_to_file = {
                    executor.submit(runner_fn, avelyn_bin, f, args.timeout): f
                    for f in all_test_files
                }
                for future in as_completed(future_to_file):
                    test_file = future_to_file[future]
                    test_name = os.path.basename(test_file)
                    res = future.result()
                    
                    with print_lock:
                        completed_counter += 1
                        idx = completed_counter
                        if res["ok"]:
                            passed_count += 1
                            status_str = "\033[92mPASS\033[0m" if sys.stdout.isatty() else "PASS"
                            print(f"[{idx:4d}/{total_tests:4d}] [{status_str}] {test_name:<48} ({res['time']:.3f}s)")
                        else:
                            failed_count += 1
                            status_str = "\033[91mFAIL\033[0m" if sys.stdout.isatty() else "FAIL"
                            err_msg = res["stderr"].strip() or res["stdout"].strip()
                            first_err_line = err_msg.splitlines()[0] if err_msg else "Unknown error"
                            print(f"[{idx:4d}/{total_tests:4d}] [{status_str}] {test_name:<48} ({res['time']:.3f}s) — {first_err_line[:60]}")
                            failures.append((test_name, res["error_type"], err_msg))

    elif mode == "dual":
        dual_results = []
        for idx, test_file in enumerate(all_test_files, 1):
            test_name = os.path.basename(test_file)
            res = execute_dual_test(avelyn_bin, test_file, TEMP_BINS_DIR, args.timeout)
            dual_results.append(res)
            
            if res["match"]:
                passed_count += 1
                status_str = "\033[92mPASS (DUAL MATCH)\033[0m" if sys.stdout.isatty() else "PASS (DUAL MATCH)"
                print(f"[{idx:4d}/{total_tests:4d}] [{status_str}] {test_name:<42} (I:{res['interp']['time']:.2f}s, N:{res['native']['time']:.2f}s)")
            else:
                failed_count += 1
                status_str = "\033[91mFAIL\033[0m" if sys.stdout.isatty() else "FAIL"
                print(f"[{idx:4d}/{total_tests:4d}] [{status_str}] {test_name:<42} — {res['diff_reason']}")
                failures.append((test_name, "DUAL_DIFF", res["diff_reason"]))
                if args.fail_fast:
                    print("\n[STOPPED] --fail-fast triggered on mismatch/failure.")
                    break

    elapsed = time.time() - t_start
    print(f"\n  EXECUTION SUMMARY [{mode.upper()} MODE]:")
    print(f"    * Total Executed : {passed_count + failed_count} / {total_tests}")
    print(f"    * PASSED         : {passed_count}")
    print(f"    * FAILED         : {failed_count}")
    print(f"    * Total Time     : {elapsed:.2f} seconds")

    if failures:
        print(f"\nTotal Failures ({len(failures)}):")
        for name, err_type, msg in failures:
            print(f"\n- {name} [{err_type}]:")
            print("  " + "\n  ".join(msg.splitlines()[:6]))

    # Exit code: 0 for all tests passed, 1 for any failure
    sys.exit(0 if failed_count == 0 else 1)

if __name__ == "__main__":
    main()
