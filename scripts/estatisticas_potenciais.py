import re
from collections import Counter

content = open("/home/rafael/claude/guerra-fria-jogar/assets/data/economia/potenciais.ron").read()

entries = re.findall(r'\(\s*estado_id:\s*"([^"]+)",\s*terra_aravel:\s*(\d+),\s*culturas_permitidas:\s*\[([^\]]*)\],\s*recursos:\s*\{([^}]*)\},\s*qualidade_recursos:\s*\{([^}]*)\}', content)

print(f"Total de estados no arquivo: {len(entries)}")

culturas_count = Counter()
recursos_count = Counter()
qualidades_count = Counter()
total_terra_aravel = 0

for eid, terra, cults, recs, quals in entries:
    total_terra_aravel += int(terra)
    for c in re.findall(r'"([^"]+)"', cults):
        culturas_count[c] += 1
    for r in re.findall(r'"([^"]+)":', recs):
        recursos_count[r] += 1
    for q in re.findall(r'"([^"]+)":', quals):
        qualidades_count[q] += 1

print(f"\nDistribuição de culturas permitidas ({len(culturas_count)} culturas):")
for c, count in culturas_count.most_common():
    print(f"  {c}: {count} estados")

print(f"\nDistribuição de recursos ({len(recursos_count)} recursos):")
for r, count in recursos_count.most_common():
    print(f"  {r}: {count} estados")

print(f"\nDistribuição de bônus de qualidade ({len(qualidades_count)} tipos):")
for q, count in qualidades_count.most_common():
    print(f"  {q}: {count} estados com bônus")

print(f"\nTotal de terra arável mundial: {total_terra_aravel} níveis")
