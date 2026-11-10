import subprocess
import re

proc = subprocess.run(['git', '-C', '/home/rafael/claude/hege-wt/a-distribuicao', 'show', 'jogavel:assets/data/economia/provincias.ron'], capture_output=True, text=True)
pop_t, emp_t = 0.0, 0.0
for line in proc.stdout.splitlines():
    if 'pais_id:"usa"' in line:
        for m in re.finditer(r'tipo_pop_id:"([^"]+)"[^)]*?populacao:([0-9.]+)[^)]*?empregados:([0-9.]+)', line):
            tp, pop, emp = m.group(1), float(m.group(2)), float(m.group(3))
            if tp != 'camponeses':
                pop_t += pop
                emp_t += emp
print(f'JOGAVEL: pop={pop_t}, emp={emp_t}, unemp={(pop_t - emp_t)/pop_t if pop_t else 0:.4f}')

# Now check current generated provincias.ron
pop_t2, emp_t2 = 0.0, 0.0
with open('/home/rafael/claude/hege-wt/a-distribuicao/assets/data/economia/provincias.ron') as f:
    for line in f:
        if 'pais_id:"usa"' in line:
            for m in re.finditer(r'tipo_pop_id:"([^"]+)"[^)]*?populacao:([0-9.]+)[^)]*?empregados:([0-9.]+)', line):
                tp, pop, emp = m.group(1), float(m.group(2)), float(m.group(3))
                if tp != 'camponeses':
                    pop_t2 += pop
                    emp_t2 += emp
print(f'CURRENT: pop={pop_t2}, emp={emp_t2}, unemp={(pop_t2 - emp_t2)/pop_t2 if pop_t2 else 0:.4f}')
