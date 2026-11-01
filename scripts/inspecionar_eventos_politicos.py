import re
from collections import Counter

path = "/home/rafael/claude/guerra-fria-staging/evolucao_ideologica/eventos_politicos.ron"
with open(path, "r", encoding="utf-8") as f:
    content = f.read()

print("File length:", len(content))

# Find all field names in CondicoesEvento
cond_blocks = re.findall(r"CondicoesEvento\s*\((.*?)\)", content, re.DOTALL)
print(f"Total CondicoesEvento blocks: {len(cond_blocks)}")
cond_fields = Counter()
for b in cond_blocks:
    # find lines with field_name:
    for line in b.split("\n"):
        line = line.strip()
        m = re.match(r"([a-zA-Z0-9_]+)\s*:", line)
        if m:
            cond_fields[m.group(1)] += 1

print("CondicoesEvento fields:")
for k, v in cond_fields.most_common():
    print(f"  {k}: {v}")

# Find all EventoPolitico fields
ev_blocks = re.findall(r"EventoPolitico\s*\((.*?)\n\s*\),", content, re.DOTALL)
print(f"Total EventoPolitico blocks: {len(ev_blocks)}")
ev_fields = Counter()
for b in ev_blocks:
    for line in b.split("\n"):
        line = line.strip()
        m = re.match(r"([a-zA-Z0-9_]+)\s*:", line)
        if m:
            ev_fields[m.group(1)] += 1
print("EventoPolitico fields:")
for k, v in ev_fields.most_common():
    print(f"  {k}: {v}")

# Find all Modificador ids
mod_ids = Counter(re.findall(r'Modificador\s*\(\s*id\s*:\s*"([^"]+)"', content))
print(f"Total distinct Modificador ids: {len(mod_ids)}")
print("Sample Modificador ids:")
for k, v in mod_ids.most_common(40):
    print(f"  {k}: {v}")
