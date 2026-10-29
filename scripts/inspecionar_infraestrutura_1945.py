import re
from collections import defaultdict

provincias_path = "/home/rafael/claude/guerra-fria-jogar/assets/data/economia/provincias.ron"

pais_counts = defaultdict(lambda: defaultdict(int))
total_provs = 0

with open(provincias_path, "r", encoding="utf-8") as f:
    for line in f:
        line = line.strip()
        if not line.startswith("(id:"):
            continue
        total_provs += 1
        m_pais = re.search(r'pais_id:"([^"]+)"', line)
        pid = m_pais.group(1) if m_pais else "desc"
        
        # encontrar todos os tipo_predio_id:"...",nivel:X
        for m in re.finditer(r'tipo_predio_id:"([^"]+)",nivel:(\d+)', line):
            tipo = m.group(1)
            nivel = int(m.group(2))
            pais_counts[pid][tipo] += nivel

print(f"Total de províncias analisadas: {total_provs}")
print("\n--- Prédios por País Chave em 1945 ---")
infra_types = {"rede_ferroviaria", "rodovia", "porto_comercial"}
for pid in ["usa", "gbr", "fra", "bel", "deu", "sov", "bra", "nga", "cog", "ang", "ind"]:
    predios = pais_counts[pid]
    ferrovia = predios.get("rede_ferroviaria", 0)
    porto = predios.get("porto_comercial", 0)
    rodovia = predios.get("rodovia", 0)
    total_outros = sum(v for k, v in predios.items() if k not in infra_types)
    print(f"País {pid.upper():<5}: Ferrovias={ferrovia:<5} Portos={porto:<5} Rodovias={rodovia:<6} | Outros produtivos={total_outros:<6}")
