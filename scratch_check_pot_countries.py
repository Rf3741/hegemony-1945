import os
import re
from collections import Counter

os.chdir("/home/rafael/claude/hege-wt/a-distribuicao")

with open("assets/data/economia/potenciais.ron", "r", encoding="utf-8") as f:
    text = f.read()

country_counts = Counter()
for m in re.finditer(r'estado_id:\s*"([a-z]{3})_', text):
    country_counts[m.group(1)] += 1

print("Countries in potenciais.ron:", country_counts)
