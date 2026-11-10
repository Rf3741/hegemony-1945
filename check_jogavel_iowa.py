import subprocess
import re

proc = subprocess.run(['git', '-C', '/home/rafael/claude/hege-wt/a-distribuicao', 'show', 'jogavel:assets/data/economia/provincias.ron'], capture_output=True, text=True)

for line in proc.stdout.splitlines():
    if 'Iowa' in line and 'pais_id:"usa"' in line:
        print("JOGAVEL Iowa:")
        for m in re.finditer(r'tipo_predio_id:"([^"]+)"[^)]*?nivel:([0-9]+)', line):
            print(f"  {m.group(1):30}: {m.group(2)}")
