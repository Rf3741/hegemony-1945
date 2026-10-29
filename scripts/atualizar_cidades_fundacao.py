import re

caminho = "/home/rafael/claude/guerra-fria-jogar/assets/data/territorio/cidades.ron"

with open(caminho, "r", encoding="utf-8") as f:
    conteudo = f.read()

# 1. Brasília
alvo_brasilia = """    "brasilia": (
        id: "brasilia",
        nome: "Brasília",
        populacao_1945: 34200,
        latitude: -15.7814,
        longitude: -47.9180,
        estado_id: "bra_federal",
        pais_tag: "BRA",
        eh_capital_estado: true,
        eh_porto: true,
    ),"""

novo_brasilia = """    "brasilia": (
        id: "brasilia",
        nome: "Brasília",
        populacao_1945: 34200,
        latitude: -15.7814,
        longitude: -47.9180,
        estado_id: "bra_federal",
        pais_tag: "BRA",
        eh_capital_estado: true,
        eh_porto: false,
        ano_fundacao: Some(1960),
    ),"""

assert alvo_brasilia in conteudo, "Brasília não encontrada para substituição"
conteudo = conteudo.replace(alvo_brasilia, novo_brasilia)

# 2. Islamabad
alvo_islamabad = """    "islamabad": (
        id: "islamabad",
        nome: "Islamabad",
        populacao_1945: 34200,
        latitude: 33.7019,
        longitude: 73.1647,
        estado_id: "raj_norte",
        pais_tag: "RAJ",
        eh_capital_estado: true,
        eh_porto: true,
    ),"""

novo_islamabad = """    "islamabad": (
        id: "islamabad",
        nome: "Islamabad",
        populacao_1945: 34200,
        latitude: 33.7019,
        longitude: 73.1647,
        estado_id: "raj_norte",
        pais_tag: "RAJ",
        eh_capital_estado: true,
        eh_porto: false,
        ano_fundacao: Some(1960),
    ),"""

assert alvo_islamabad in conteudo, "Islamabad não encontrada para substituição"
conteudo = conteudo.replace(alvo_islamabad, novo_islamabad)

# 3. Belmopan
alvo_belmopan = """    "belmopan": (
        id: "belmopan",
        nome: "Belmopan",
        populacao_1945: 10000,
        latitude: 17.2520,
        longitude: -88.7671,
        estado_id: "blz_belize",
        pais_tag: "BLZ",
        eh_capital_estado: true,
        eh_porto: true,
    ),"""

novo_belmopan = """    "belmopan": (
        id: "belmopan",
        nome: "Belmopan",
        populacao_1945: 10000,
        latitude: 17.2520,
        longitude: -88.7671,
        estado_id: "blz_belize",
        pais_tag: "BLZ",
        eh_capital_estado: true,
        eh_porto: false,
        ano_fundacao: Some(1970),
    ),"""

assert alvo_belmopan in conteudo, "Belmopan não encontrada para substituição"
conteudo = conteudo.replace(alvo_belmopan, novo_belmopan)

# 4. Volgogrado -> Stalingrado (depois Volgogrado em 1961)
alvo_volgogrado = """    "volgogrado": (
        id: "volgogrado",
        nome: "Volgogrado",
        populacao_1945: 437950,
        latitude: 48.7120,
        longitude: 44.4980,
        estado_id: "sov_chernozem_central",
        pais_tag: "SOV",
        eh_capital_estado: true,
        eh_porto: true,
    ),"""

novo_volgogrado = """    "volgogrado": (
        id: "volgogrado",
        nome: "Stalingrado",
        populacao_1945: 437950,
        latitude: 48.7120,
        longitude: 44.4980,
        estado_id: "sov_chernozem_central",
        pais_tag: "SOV",
        eh_capital_estado: true,
        eh_porto: true,
        mudancas_nome: [(1961, "Volgogrado")],
    ),"""

assert alvo_volgogrado in conteudo, "Volgogrado não encontrada para substituição"
conteudo = conteudo.replace(alvo_volgogrado, novo_volgogrado)

# 5. São Petersburgo -> Leningrado (depois São Petersburgo em 1991)
alvo_spb = """    "sao_petersburgo_sov_le": (
        id: "sao_petersburgo_sov_le",
        nome: "São Petersburgo",
        populacao_1945: 2757850,
        latitude: 59.9410,
        longitude: 30.3141,
        estado_id: "sov_leningrado",
        pais_tag: "SOV",
        eh_capital_estado: true,
        eh_porto: true,
    ),"""

novo_spb = """    "sao_petersburgo_sov_le": (
        id: "sao_petersburgo_sov_le",
        nome: "Leningrado",
        populacao_1945: 2757850,
        latitude: 59.9410,
        longitude: 30.3141,
        estado_id: "sov_leningrado",
        pais_tag: "SOV",
        eh_capital_estado: true,
        eh_porto: true,
        mudancas_nome: [(1991, "São Petersburgo")],
    ),"""

assert alvo_spb in conteudo, "São Petersburgo não encontrada para substituição"
conteudo = conteudo.replace(alvo_spb, novo_spb)

# 6. Chandigarh (inserir antes de changchun)
alvo_changchun = """    "changchun": ("""
novo_chandigarh = """    "chandigarh": (
        id: "chandigarh",
        nome: "Chandigarh",
        populacao_1945: 50000,
        latitude: 30.7333,
        longitude: 76.7794,
        estado_id: "raj_norte",
        pais_tag: "RAJ",
        eh_capital_estado: true,
        eh_porto: false,
        ano_fundacao: Some(1953),
    ),
    "changchun": ("""

assert alvo_changchun in conteudo, "Changchun não encontrada para inserção de Chandigarh"
conteudo = conteudo.replace(alvo_changchun, novo_chandigarh)

with open(caminho, "w", encoding="utf-8") as f:
    f.write(conteudo)

print("Cidades atualizadas com sucesso em cidades.ron!")
