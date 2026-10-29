import re
from collections import Counter

content = open("/home/rafael/claude/guerra-fria-jogar/assets/data/economia/provincias.ron").read()

prov_matches = re.findall(r'id:"([^"]+)",nome:"([^"]+)",(?:codigo_externo:Some\("([^"]+)"\),)?estado_id:"([^"]+)",pais_id:"([^"]+)"', content)
print(f"Total províncias: {len(prov_matches)}")

# Let's count buildings
predios = Counter(re.findall(r'tipo_predio_id:"([^"]+)"', content))
print(f"\nTipos de prédio em provincias.ron ({len(predios)} tipos):")
for pid, c in predios.most_common():
    print(f"  {pid}: {c} ocorrências")
