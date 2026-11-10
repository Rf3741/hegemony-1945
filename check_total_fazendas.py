import subprocess
import re

proc = subprocess.run(['git', '-C', '/home/rafael/claude/hege-wt/a-distribuicao', 'show', 'jogavel:assets/data/economia/provincias.ron'], capture_output=True, text=True)

total_fazendas = 0
for line in proc.stdout.splitlines():
    if 'pais_id:"usa"' in line:
        m = re.search(r'tipo_predio_id:"fazenda_graos"[^)]*?nivel:([0-9]+)', line)
        if m:
            total_fazendas += int(m.group(1))

print("Total fazenda_graos in USA (jogavel):", total_fazendas)
