#!/usr/bin/env python3
"""
run_all_tests.py — Comprehensive Test Runner for Sylvel (Avelyn)

Discovers and executes every `.lyn` test file under the Tests/ folder.
Supports both Interpreter mode and Native LLVM compilation mode.
Summarizes results and details all failures at the end.
"""

import sys
import os
import glob
import subprocess
import time
import argparse
from pathlib import Path

# Ensure UTF-8 output formatting on Windows
if sys.platform == "win32":
    try:
        sys.stdout.reconfigure(encoding='utf-8', errors='replace')
        sys.stderr.reconfigure(encoding='utf-8', errors='replace')
    except Exception:
        pass

def find_avelyn_bin(provided_path=None):
    if provided_path and os.path.exists(provided_path):
        return os.path.abspath(provided_path)

    candidates = [
        os.path.abspath("avelyn/target/release/avelyn.exe"),
        os.path.abspath("avelyn/target/release/avelyn"),
        os.path.abspath("avelyn/target/debug/avelyn.exe"),
        os.path.abspath("avelyn/target/debug/avelyn"),
        "avelyn.exe",
        "avelyn"
    ]

    for cand in candidates:
        if os.path.exists(cand):
            return cand
    return "avelyn"

def run_test_interpreter(avelyn_bin, file_path):
    cmd = [avelyn_bin, file_path]
    start = time.time()
    try:
        proc = subprocess.run(
            cmd,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
            encoding="utf-8",
            errors="replace"
        )
        elapsed = time.time() - start
        success = (proc.returncode == 0)
        return success, proc.stdout, proc.stderr, elapsed, proc.returncode
    except Exception as e:
        elapsed = time.time() - start
        return False, "", str(e), elapsed, -1

def run_test_native_llvm(avelyn_bin, file_path, temp_dir):
    stem = Path(file_path).stem
    nonce = int(time.time() * 1000000)
    out_exe = os.path.join(temp_dir, f"{stem}_{nonce}.exe" if sys.platform == "win32" else f"{stem}_{nonce}")
    
    # 1. Compile via LLVM
    compile_cmd = [avelyn_bin, "compile", file_path, "-o", out_exe]
    start = time.time()
    try:
        c_proc = subprocess.run(
            compile_cmd,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
            encoding="utf-8",
            errors="replace"
        )
        if c_proc.returncode != 0:
            elapsed = time.time() - start
            err_msg = f"LLVM Compilation Failed (code {c_proc.returncode}):\nSTDOUT:\n{c_proc.stdout}\nSTDERR:\n{c_proc.stderr}"
            return False, c_proc.stdout, err_msg, elapsed, c_proc.returncode
    except Exception as e:
        elapsed = time.time() - start
        return False, "", f"Compilation Exception: {e}", elapsed, -1

    # 2. Execute compiled native binary
    try:
        r_proc = subprocess.run(
            [out_exe],
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
            encoding="utf-8",
            errors="replace"
        )
        elapsed = time.time() - start
        success = (r_proc.returncode == 0)
        return success, r_proc.stdout, r_proc.stderr, elapsed, r_proc.returncode
    except Exception as e:
        elapsed = time.time() - start
        return False, "", f"Execution Exception: {e}", elapsed, -1
    finally:
        if os.path.exists(out_exe):
            try:
                os.remove(out_exe)
            except Exception:
                pass

def main():
    parser = argparse.ArgumentParser(description="Sylvel / Avelyn Test Suite Runner")
    parser.add_argument("--bin", type=str, help="Path to avelyn binary executable")
    parser.add_argument("--dir", type=str, default="Tests", help="Directory containing .lyn test files")
    parser.add_argument("--mode", choices=["interpreter", "native", "both"], default="interpreter", help="Execution mode (interpreter, native LLVM, or both)")
    args = parser.parse_args()

    avelyn_bin = find_avelyn_bin(args.bin)
    test_dir = os.path.abspath(args.dir)

    if not os.path.exists(test_dir):
        print(f"Error: Test directory '{test_dir}' not found.", flush=True)
        sys.exit(1)

    test_files = sorted(glob.glob(os.path.join(test_dir, "**", "*.lyn"), recursive=True))

    if not test_files:
        print(f"No .lyn test files found in '{test_dir}'.", flush=True)
        sys.exit(0)

    print("=" * 80, flush=True)
    print(f"  Avelyn Test Suite Runner", flush=True)
    print(f"  Binary   : {avelyn_bin}", flush=True)
    print(f"  Test Dir : {test_dir}", flush=True)
    print(f"  Mode     : {args.mode}", flush=True)
    print(f"  Found    : {len(test_files)} test file(s)", flush=True)
    print("=" * 80, flush=True)
    print(flush=True)

    temp_dir = os.path.abspath("scratch/test_builds")
    os.makedirs(temp_dir, exist_ok=True)

    modes_to_run = []
    if args.mode in ["interpreter", "both"]:
        modes_to_run.append("interpreter")
    if args.mode in ["native", "both"]:
        modes_to_run.append("native")

    total_runs = 0
    passed_runs = 0
    failed_runs = 0
    failures = []

    for mode in modes_to_run:
        print(f"--- Running Tests in [{mode.upper()}] Mode ---", flush=True)
        for idx, file_path in enumerate(test_files, 1):
            rel_name = os.path.relpath(file_path, test_dir)
            total_runs += 1

            if mode == "interpreter":
                ok, stdout, stderr, elapsed, code = run_test_interpreter(avelyn_bin, file_path)
            else:
                ok, stdout, stderr, elapsed, code = run_test_native_llvm(avelyn_bin, file_path, temp_dir)

            if ok:
                passed_runs += 1
                status_str = "PASSED"
                print(f"[{idx:3d}/{len(test_files):3d}] {rel_name:<45} ... {status_str} ({elapsed:.2f}s)", flush=True)
            else:
                failed_runs += 1
                status_str = "FAILED"
                print(f"[{idx:3d}/{len(test_files):3d}] {rel_name:<45} ... {status_str} ({elapsed:.2f}s)", flush=True)
                failures.append({
                    "mode": mode,
                    "file": file_path,
                    "rel_name": rel_name,
                    "code": code,
                    "stdout": stdout,
                    "stderr": stderr
                })

        print(flush=True)

    print("=" * 80, flush=True)
    print("                      TEST EXECUTION SUMMARY", flush=True)
    print("=" * 80, flush=True)
    print(f"Total Executed : {total_runs}", flush=True)
    print(f"PASSED         : {passed_runs}", flush=True)
    print(f"FAILED         : {failed_runs}", flush=True)
    print("=" * 80, flush=True)

    if failures:
        print("\n" + "!" * 80, flush=True)
        print("                        DETAILED FAILURE REPORT", flush=True)
        print("!" * 80 + "\n", flush=True)

        for f_idx, fail in enumerate(failures, 1):
            print(f"Failure #{f_idx}: [{fail['mode'].upper()}] {fail['rel_name']}", flush=True)
            print(f"File Path   : {fail['file']}", flush=True)
            print(f"Exit Code   : {fail['code']}", flush=True)
            if fail['stdout'].strip():
                print("--- STDOUT ---", flush=True)
                print(fail['stdout'].strip(), flush=True)
            if fail['stderr'].strip():
                print("--- STDERR ---", flush=True)
                print(fail['stderr'].strip(), flush=True)
            print("-" * 80, flush=True)
            print(flush=True)

        sys.exit(1)
    else:
        print("\nAll tests completed successfully with zero failures!", flush=True)
        sys.exit(0)

if __name__ == "__main__":
    main()
