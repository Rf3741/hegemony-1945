import subprocess
import re

proc = subprocess.run(['git', '-C', '/home/rafael/claude/hege-wt/a-distribuicao', 'show', 'fcd68207:assets/data/economia/provincias.ron'], capture_output=True, text=True)

total_vagas_jog = 0
total_emp_jog = 0
total_pop_jog = 0
for line in proc.stdout.splitlines():
    if 'pais_id:"usa"' in line:
        for m in re.finditer(r'tipo_pop_id:"([^"]+)"[^)]*?populacao:([0-9.]+)[^)]*?empregados:([0-9.]+)', line):
            tp, pop, emp = m.group(1), float(m.group(2)), float(m.group(3))
            total_pop_jog += pop
            if tp != 'camponeses':
                total_emp_jog += emp
        for bm in re.finditer(r'empregados_por_tipo:\{([^}]+)\}', line):
            total_vagas_jog += sum(float(x.group(1)) for x in re.finditer(r':([0-9.]+)', bm.group(1)))

print(f"JOGAVEL: pop={total_pop_jog} emp={total_emp_jog} vagas={total_vagas_jog}")
