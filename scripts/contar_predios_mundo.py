import re
from collections import defaultdict

provincias_path = "/home/rafael/claude/guerra-fria-jogar/assets/data/economia/provincias.ron"

predios_globais = defaultdict(int)

with open(provincias_path, "r", encoding="utf-8") as f:
    for line in f:
        for m in re.finditer(r'tipo_predio_id:"([^"]+)",nivel:(\d+)', line):
            tipo = m.group(1)
            nivel = int(m.group(2))
            predios_globais[tipo] += nivel

print("Total de níveis de prédios por tipo no mundo (1945):")
for k, v in sorted(predios_globais.items(), key=lambda x: -x[1])[:25]:
    print(f"  {k:<32}: {v}")
