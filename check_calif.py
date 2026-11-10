import subprocess
import re

proc = subprocess.run(['git', '-C', '/home/rafael/claude/hege-wt/a-distribuicao', 'show', 'jogavel:assets/data/economia/provincias.ron'], capture_output=True, text=True)

for line in proc.stdout.splitlines():
    if 'Califórnia' in line:
        p_pop = 0.0
        p_emp = 0.0
        for m in re.finditer(r'tipo_pop_id:"([^"]+)"[^)]*?populacao:([0-9.]+)[^)]*?empregados:([0-9.]+)', line):
            tp, pop, emp = m.group(1), float(m.group(2)), float(m.group(3))
            p_pop += pop
            if tp != 'camponeses':
                p_emp += emp
        p_vagas = 0.0
        for bm in re.finditer(r'empregados_por_tipo:\{([^}]+)\}', line):
            p_vagas += sum(float(x.group(1)) for x in re.finditer(r':([0-9.]+)', bm.group(1)))
        print(f"JOGAVEL California: pop={p_pop} emp={p_emp} vagas={p_vagas}")

with open('/home/rafael/claude/hege-wt/a-distribuicao/assets/data/economia/provincias.ron') as f:
    for line in f:
        if 'Califórnia' in line:
            p_pop = 0.0
            p_emp = 0.0
            for m in re.finditer(r'tipo_pop_id:"([^"]+)"[^)]*?populacao:([0-9.]+)[^)]*?empregados:([0-9.]+)', line):
                tp, pop, emp = m.group(1), float(m.group(2)), float(m.group(3))
                p_pop += pop
                if tp != 'camponeses':
                    p_emp += emp
            p_vagas = 0.0
            for bm in re.finditer(r'empregados_por_tipo:\{([^}]+)\}', line):
                p_vagas += sum(float(x.group(1)) for x in re.finditer(r':([0-9.]+)', bm.group(1)))
            print(f"CURRENT California: pop={p_pop} emp={p_emp} vagas={p_vagas}")
