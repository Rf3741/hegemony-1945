import os

staging = "/home/rafael/claude/guerra-fria-staging"
matches = []
for root, dirs, files in os.walk(staging):
    for f in files:
        if f.endswith(".ron"):
            p = os.path.join(root, f)
            try:
                with open(p, "r", encoding="utf-8", errors="ignore") as fl:
                    s = fl.read()
                    if "CondicoesEvento" in s or "EventoPolitico" in s:
                        matches.append(p)
            except Exception as e:
                pass

print("Files matching in staging:")
for m in matches:
    print(" ", m)
