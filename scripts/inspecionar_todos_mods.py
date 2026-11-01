import re
from collections import Counter

path = "/home/rafael/claude/guerra-fria-staging/evolucao_ideologica/eventos_politicos.ron"
with open(path, "r", encoding="utf-8") as f:
    content = f.read()

mod_ids = Counter(re.findall(r'Modificador\s*\(\s*id\s*:\s*"([^"]+)"', content))
print("All 62 Modificador ids:")
for k, v in sorted(mod_ids.items()):
    print(f"  {k}: {v}")
