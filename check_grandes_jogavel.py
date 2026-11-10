import subprocess
import re

proc = subprocess.run(['git', '-C', '/home/rafael/claude/hege-wt/a-distribuicao', 'show', 'fcd68207:assets/data/economia/provincias.ron'], capture_output=True, text=True)

GRANDES = ["usa", "sov", "gbr", "fra", "ger", "jap", "ind", "chi", "bra", "ita"]

for p in GRANDES:
    pop_t, emp_t = 0.0, 0.0
    for line in proc.stdout.splitlines():
        if f'pais_id:"{p}"' in line:
            for m in re.finditer(r'tipo_pop_id:"([^"]+)"[^)]*?populacao:([0-9.]+)[^)]*?empregados:([0-9.]+)', line):
                tp, pop, emp = m.group(1), float(m.group(2)), float(m.group(3))
                if tp != 'camponeses':
                    pop_t += pop
                    emp_t += emp
    unemp = (pop_t - emp_t) / pop_t if pop_t else 0
    print(f"{p:5} pop={pop_t:10.0f} emp={emp_t:10.0f} unemp={unemp*100:.2f}%")
