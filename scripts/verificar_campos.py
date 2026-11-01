import re

path = "/home/rafael/claude/guerra-fria-staging/evolucao_ideologica/eventos_politicos.ron"
with open(path, "r", encoding="utf-8") as f:
    text = f.read()

# Let's verify every CondicoesEvento
for m in re.finditer(r"CondicoesEvento\s*\((.*?)\)", text, re.DOTALL):
    inner = m.group(1)
    keys = re.findall(r"([a-zA-Z0-9_]+)\s*:", inner)
    for k in keys:
        if k not in ["leis_obrigatorias", "leis_proibidas", "legitimidade_maxima"]:
            print("Found other key in CondicoesEvento:", k)

print("Checked all CondicoesEvento.")

# Check all fields in OpcaoEventoPolitico
for m in re.finditer(r"OpcaoEventoPolitico\s*\((.*?)\n\s*\)", text, re.DOTALL):
    inner = m.group(1)
    keys = re.findall(r"([a-zA-Z0-9_]+)\s*:", inner)
    for k in keys:
        if k not in ["id", "texto", "descricao_resultado", "efeitos", "ia_peso"]:
            print("Found other key in OpcaoEventoPolitico:", k)

print("Checked all OpcaoEventoPolitico.")
