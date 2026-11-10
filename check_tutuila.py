with open('/home/rafael/claude/hege-wt/a-distribuicao/assets/data/economia/provincias.ron') as f:
    for line in f:
        if 'Tutuila Oriental' in line or 'Montana' in line:
            import re
            m_nome = re.search(r'nome:"([^"]+)"', line)
            print("---", m_nome.group(1), "---")
            for m in re.finditer(r'tipo_predio_id:"([^"]+)"[^)]*?nivel:([0-9]+)', line):
                if int(m.group(2)) > 10:
                    print("  ", m.group(1), "nivel:", m.group(2))
