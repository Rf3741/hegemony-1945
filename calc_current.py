import re

total_vagas = 0
total_emp = 0
total_pop = 0
with open('/home/rafael/claude/hege-wt/a-distribuicao/assets/data/economia/provincias.ron') as f:
    for line in f:
        if 'pais_id:"usa"' in line:
            for m in re.finditer(r'tipo_pop_id:"([^"]+)"[^)]*?populacao:([0-9.]+)[^)]*?empregados:([0-9.]+)', line):
                tp, pop, emp = m.group(1), float(m.group(2)), float(m.group(3))
                total_pop += pop
                if tp != 'camponeses':
                    total_emp += emp
            for bm in re.finditer(r'empregados_por_tipo:\{([^}]+)\}', line):
                total_vagas += sum(float(x.group(1)) for x in re.finditer(r':([0-9.]+)', bm.group(1)))

print(f"CURRENT: pop={total_pop} emp={total_emp} vagas={total_vagas}")
