import subprocess
import re

proc = subprocess.run(['git', '-C', '/home/rafael/claude/hege-wt/a-distribuicao', 'show', 'jogavel:assets/data/economia/provincias.ron'], capture_output=True, text=True)

for line in proc.stdout.splitlines():
    if 'Nova Iorque' in line and 'pais_id:"usa"' in line:
        print("JOGAVEL Nova Iorque:")
        tot_v = 0
        tot_lvl = 0
        for m in re.finditer(r'tipo_predio_id:"([^"]+)"[^)]*?nivel:([0-9]+)[^)]*?empregados_por_tipo:\{([^}]+)\}', line):
            lvl = int(m.group(2))
            vagas = sum(float(x.group(1)) for x in re.finditer(r':([0-9.]+)', m.group(3)))
            tot_lvl += lvl
            tot_v += vagas
            if lvl > 100:
                print(f"  {m.group(1):30}: nivel={lvl} vagas={vagas:.0f}")
        print(f"Total levels: {tot_lvl}, total vagas: {tot_v:.0f}")
