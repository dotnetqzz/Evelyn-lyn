#!/usr/bin/env python3
import subprocess, os, glob, time
from pathlib import Path

root = Path('.')
avelyn_candidates = [
    os.path.abspath('avelyn/target/release/avelyn.exe'),
    os.path.abspath('avelyn/target/release/avelyn'),
    os.path.abspath('avelyn/target/debug/avelyn.exe'),
    os.path.abspath('avelyn/target/debug/avelyn'),
    'avelyn.exe', 'avelyn'
]
avelyn = None
for c in avelyn_candidates:
    if os.path.exists(c):
        avelyn = c; break
if not avelyn:
    avelyn = 'avelyn'

tests = sorted(glob.glob('Tests/**/*.lyn', recursive=True))
out_dir = Path('scratch/compare_outs')
interp_dir = out_dir / 'interpreter'
native_dir = out_dir / 'native'
interp_dir.mkdir(parents=True, exist_ok=True)
native_dir.mkdir(parents=True, exist_ok=True)

mismatches = []

for t in tests:
    stem = Path(t).stem
    # interpreter
    try:
        p = subprocess.run([avelyn, t], stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True, encoding='utf-8', errors='replace')
        interp_out = p.stdout
        interp_err = p.stderr
        interp_code = p.returncode
    except Exception as e:
        interp_out = ''
        interp_err = str(e)
        interp_code = -1
    open(interp_dir / f"{stem}.out", 'w', encoding='utf-8').write(interp_out)
    open(interp_dir / f"{stem}.err", 'w', encoding='utf-8').write(interp_err)
    open(interp_dir / f"{stem}.code", 'w', encoding='utf-8').write(str(interp_code))

    # native compile+run
    nonce = int(time.time()*1000000)
    out_exe = str(Path('scratch') / f"{stem}_{nonce}.exe")
    try:
        c = subprocess.run([avelyn, 'compile', t, '-o', out_exe], stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True, encoding='utf-8', errors='replace')
        if c.returncode != 0:
            native_out = c.stdout
            native_err = 'COMPILE_ERR:\n' + c.stderr
            native_code = c.returncode
        else:
            r = subprocess.run([out_exe], stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True, encoding='utf-8', errors='replace')
            native_out = r.stdout
            native_err = r.stderr
            native_code = r.returncode
    except Exception as e:
        native_out = ''
        native_err = str(e)
        native_code = -1
    open(native_dir / f"{stem}.out", 'w', encoding='utf-8').write(native_out)
    open(native_dir / f"{stem}.err", 'w', encoding='utf-8').write(native_err)
    open(native_dir / f"{stem}.code", 'w', encoding='utf-8').write(str(native_code))

    # compare
    if interp_out != native_out or interp_err != native_err or interp_code != native_code:
        mismatches.append((t, interp_code, native_code))

    # cleanup exe
    try:
        if os.path.exists(out_exe): os.remove(out_exe)
    except:
        pass

# report
print('Total tests:', len(tests))
print('Mismatches:', len(mismatches))
for m in mismatches:
    print(m)

if mismatches:
    exit(1)
else:
    exit(0)
