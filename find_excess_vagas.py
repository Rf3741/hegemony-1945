import re

with open('/home/rafael/claude/hege-wt/a-distribuicao/assets/data/economia/provincias.ron') as f:
    for line in f:
        if 'pais_id:"usa"' in line:
            m_nome = re.search(r'nome:"([^"]+)"', line)
            nome = m_nome.group(1) if m_nome else '?'
            p_pop = 0.0
            p_emp = 0.0
            for m in re.finditer(r'tipo_pop_id:"([^"]+)"[^)]*?populacao:([0-9.]+)[^)]*?empregados:([0-9.]+)', line):
                tp, pop, emp = m.group(1), float(m.group(2)), float(m.group(3))
                if tp != 'camponeses':
                    p_pop += pop
                    p_emp += emp
            p_vagas = 0.0
            for bm in re.finditer(r'empregados_por_tipo:\{([^}]+)\}', line):
                p_vagas += sum(float(x.group(1)) for x in re.finditer(r':([0-9.]+)', bm.group(1)))
            if p_vagas > p_pop * 1.2:
                print(f"{nome:25} pop={p_pop:10.0f} vagas={p_vagas:10.0f} excess={p_vagas - p_pop:10.0f}")
