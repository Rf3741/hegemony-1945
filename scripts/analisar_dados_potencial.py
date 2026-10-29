import re
import collections

# Path to canonico
canonico_path = "/home/rafael/claude/guerra-fria-staging/provincias_1945/regioes_1945_canonico.ron"

recursos_set = collections.Counter()
predios_set = collections.Counter()

with open(canonico_path, "r", encoding="utf-8") as f:
    for line in f:
        m_rec = re.search(r'recursos:\s*\{([^}]*)\}', line)
        if m_rec:
            items = m_rec.group(1).split(",")
            for it in items:
                parts = it.split(":")
                if len(parts) == 2:
                    k = parts[0].strip().strip('"')
                    recursos_set[k] += 1
        m_pred = re.search(r'predios:\s*\{([^}]*)\}', line)
        if m_pred:
            items = m_pred.group(1).split(",")
            for it in items:
                parts = it.split(":")
                if len(parts) == 2:
                    k = parts[0].strip().strip('"')
                    predios_set[k] += 1

print("Recursos encontrados em regioes_1945_canonico.ron:")
for k, count in recursos_set.most_common():
    print(f"  {k}: {count}")

print("\nPredios encontrados em regioes_1945_canonico.ron:")
for k, count in predios_set.most_common():
    print(f"  {k}: {count}")
