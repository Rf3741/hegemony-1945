import re

with open('/home/rafael/claude/hege-wt/a-distribuicao/assets/data/economia/provincias.ron') as f:
    for line in f:
        if 'pais_id:"usa"' in line and 'Nova Iorque' in line:
            print("NY:", line[:300])
            for m in re.finditer(r'tipo_predio_id:"([^"]+)"[^)]*?nivel:([0-9]+)[^)]*?empregados_por_tipo:\{([^}]+)\}', line):
                print("Predio:", m.group(1), "nivel:", m.group(2), "vagas:", m.group(3))
            break
