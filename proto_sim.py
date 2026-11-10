import subprocess
import re
from collections import defaultdict

# 1. Load jogavel
proc = subprocess.run(['git', '-C', '/home/rafael/claude/hege-wt/a-distribuicao', 'show', 'jogavel:assets/data/economia/provincias.ron'], capture_output=True, text=True)

# Parse provinces
provs = []
for line in proc.stdout.splitlines():
    if not line.strip() or line.strip().startswith('//'):
        continue
    # extract pais_id
    m_pais = re.search(r'pais_id:"([^"]+)"', line)
    if not m_pais:
        continue
    pais = m_pais.group(1)
    if pais != 'usa':
        continue
    m_id = re.search(r'id:"([^"]+)"', line)
    m_nome = re.search(r'nome:"([^"]+)"', line)
    m_est = re.search(r'estado_id:"([^"]+)"', line)
    m_terra = re.search(r'terra_aravel_total:([0-9]+)', line)
    
    # pops
    pops = []
    for m in re.finditer(r'tipo_pop_id:"([^"]+)"[^)]*?populacao:([0-9.]+)[^)]*?empregados:([0-9.]+)', line):
        pops.append((m.group(1), float(m.group(2)), float(m.group(3))))
    
    # predios
    predios = []
    for m in re.finditer(r'tipo_predio_id:"([^"]+)"[^)]*?nivel:([0-9]+)', line):
        predios.append((m.group(1), int(m.group(2))))
    
    # recursos
    recursos = {}
    m_rec = re.search(r'limites_recursos:\{([^}]+)\}', line)
    if m_rec:
        for rm in re.finditer(r'"([^"]+)":([0-9]+)', m_rec.group(1)):
            recursos[rm.group(1)] = int(rm.group(2))
            
    provs.append({
        'id': m_id.group(1),
        'nome': m_nome.group(1),
        'estado_id': m_est.group(1),
        'terra': int(m_terra.group(1)) if m_terra else 0,
        'pops': pops,
        'predios': predios,
        'recursos': recursos,
    })

print(f"Loaded {len(provs)} USA provinces from jogavel")
