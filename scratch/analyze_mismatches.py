#!/usr/bin/env python3
import os, glob, difflib
from pathlib import Path
interp_dir = Path('scratch/compare_outs/interpreter')
native_dir = Path('scratch/compare_outs/native')

tests = sorted([p.stem for p in interp_dir.glob('*.out')])
stdout_diff = []
stderr_diff = []
code_diff = []

for t in tests:
    iout = (interp_dir / f"{t}.out").read_text(encoding='utf-8', errors='replace')
    ierr = (interp_dir / f"{t}.err").read_text(encoding='utf-8', errors='replace')
    icod = (interp_dir / f"{t}.code").read_text(encoding='utf-8', errors='replace').strip()

    nout = (native_dir / f"{t}.out").read_text(encoding='utf-8', errors='replace')
    nerr = (native_dir / f"{t}.err").read_text(encoding='utf-8', errors='replace')
    ncod = (native_dir / f"{t}.code").read_text(encoding='utf-8', errors='replace').strip()

    if iout != nout:
        stdout_diff.append(t)
    if ierr != nerr:
        stderr_diff.append(t)
    if icod != ncod:
        code_diff.append((t, icod, ncod))

print('Total tests checked:', len(tests))
print('stdout diffs:', len(stdout_diff))
print('stderr diffs:', len(stderr_diff))
print('exit-code diffs:', len(code_diff))

# prioritize tests with exit-code diffs then stderr diffs then stdout diffs
priority = [x[0] for x in code_diff] + [t for t in stderr_diff if t not in [x[0] for x in code_diff]] + [t for t in stdout_diff if t not in [x[0] for x in code_diff] and t not in stderr_diff]

print('\nPRIORITY LIST (top 20):')
for t in priority[:20]:
    print(t)

# print small unified diffs for top 10
print('\nDIFF SUMMARIES (top 10)')
for t in priority[:10]:
    print('\n---', t)
    iout = (interp_dir / f"{t}.out").read_text(encoding='utf-8', errors='replace').splitlines()
    nout = (native_dir / f"{t}.out").read_text(encoding='utf-8', errors='replace').splitlines()
    diff = list(difflib.unified_diff(iout, nout, fromfile='interpreter', tofile='native', lineterm=''))
    if len(diff) > 200:
        diff = diff[:200]
        diff.append('...truncated...')
    print('\n'.join(diff) if diff else '(stdout identical)')

    ierr = (interp_dir / f"{t}.err").read_text(encoding='utf-8', errors='replace').splitlines()
    nerr = (native_dir / f"{t}.err").read_text(encoding='utf-8', errors='replace').splitlines()
    diff2 = list(difflib.unified_diff(ierr, nerr, fromfile='interpreter.err', tofile='native.err', lineterm=''))
    if diff2:
        if len(diff2) > 200:
            diff2 = diff2[:200]
            diff2.append('...truncated...')
        print('\nSTDERR DIFF:\n' + '\n'.join(diff2))
    else:
        print('\n(stderr identical)')

# write report
rep = Path('scratch/mismatch_report.txt')
rep.write_text(f"Total:{len(tests)}\nstdout_diffs:{len(stdout_diff)}\nstderr_diffs:{len(stderr_diff)}\ncode_diffs:{len(code_diff)}\npriority_top20:\n" + "\n".join(priority[:20]))
print('\nReport written to scratch/mismatch_report.txt')
