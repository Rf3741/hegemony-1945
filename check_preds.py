import subprocess
import re

proc = subprocess.run(['git', '-C', '/home/rafael/claude/hege-wt/a-distribuicao', 'show', 'jogavel:assets/data/economia/provincias.ron'], capture_output=True, text=True)

types_in_prov = {}
for line in proc.stdout.splitlines():
    if 'pais_id:"usa"' in line:
        m_nome = re.search(r'nome:"([^"]+)"', line)
        nome = m_nome.group(1) if m_nome else '?'
        predios = [m.group(1) for m in re.finditer(r'tipo_predio_id:"([^"]+)"', line)]
        types_in_prov[nome] = predios

print("Total USA provinces in jogavel:", len(types_in_prov))
for nome, preds in list(types_in_prov.items())[:10]:
    print(f"{nome:25}: {len(preds)} predios")
