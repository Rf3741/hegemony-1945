import re
from collections import defaultdict

estados_path = "/home/rafael/claude/guerra-fria-jogar/assets/data/economia/estados.ron"
provincias_path = "/home/rafael/claude/guerra-fria-jogar/assets/data/economia/provincias.ron"
potenciais_path = "/home/rafael/claude/guerra-fria-jogar/assets/data/economia/potenciais.ron"

# Carregar potenciais para saber se tem pesca (costeiro)
estados_com_pesca = set()
with open(potenciais_path, "r", encoding="utf-8") as f:
    pot_content = f.read()

for m in re.finditer(r'estado_id:\s*"([^"]+)"[^\)]*?recursos:\s*\{([^}]*)\}', pot_content):
    eid = m.group(1)
    rec_str = m.group(2)
    if '"pesca"' in rec_str:
        estados_com_pesca.add(eid)

print(f"Estados com pesca/costeiros: {len(estados_com_pesca)}")

# Carregar estados
estados = {}
with open(estados_path, "r", encoding="utf-8") as f:
    for m in re.finditer(r'id:\s*"([^"]+)",\s*nome:\s*"([^"]+)",\s*pais_id:\s*"([^"]+)"', f.read()):
        eid, nome, pid = m.groups()
        estados[eid] = {
            "nome": nome,
            "pais_id": pid,
            "eh_costeiro": eid in estados_com_pesca,
            "pop_urbana": 0.0,
            "uso": 0.0,
            "cap_portos": 0.0,
            "cap_ferrovias": 0.0,
            "cap_rodovias": 0.0,
            "urbanizacao_predios": 0.0,
            "num_provs": 0,
        }

# Categorias de prédios conhecidas
industriais = {"siderurgica", "fabrica_textil", "industria_alimenticia", "fabrica_ferramentas", "fabrica_maquinas", "estaleiro_naval", "fabrica_armas_leves", "fabrica_municoes", "fabrica_blindados_artilharia", "fabrica_cimento_materiais", "refinaria_petroleo", "industria_quimica"}
extrativos = {"mina_carvao", "mina_ferro", "poco_petroleo", "campo_gas_natural", "mina_uranio", "mina_cobre", "mina_bauxita", "serraria", "porto_pesca"}
agricolas = {"fazenda_graos", "rancho_pecuario", "fazenda_hortifruti", "plantacao_algodao", "plantacao_acucar", "plantacao_cafe_cha", "plantacao_tabaco", "plantacao_borracha"}
governo_militar = {"administracao_governamental", "complexo_militar_quartel", "universidade_pesquisa", "setor_construcao", "setor_servicos_comercio", "setor_bancario_financeiro", "usina_termeletrica", "usina_hidreletrica", "usina_nuclear", "central_telecomunicacoes"}

with open(provincias_path, "r", encoding="utf-8") as f:
    for line in f:
        if not line.startswith("(id:"):
            continue
        m_eid = re.search(r'estado_id:"([^"]+)"', line)
        if not m_eid:
            continue
        eid = m_eid.group(1)
        if eid not in estados:
            continue
        
        est = estados[eid]
        est["num_provs"] += 1
        
        # População urbana
        for m_pop in re.finditer(r'tipo_pop_id:"([^"]+)"[^\)]*?populacao:([\d\.]+)', line):
            tp = m_pop.group(1)
            pop = float(m_pop.group(2))
            if tp != "camponeses":
                est["pop_urbana"] += pop
                
        # Prédios
        for m_pred in re.finditer(r'tipo_predio_id:"([^"]+)",nivel:(\d+)', line):
            tipo = m_pred.group(1)
            nivel = int(m_pred.group(2))
            
            if tipo == "porto_comercial":
                if est["eh_costeiro"]:
                    est["cap_portos"] += nivel * 30.0
            elif tipo == "rede_ferroviaria":
                est["cap_ferrovias"] += nivel * 40.0
            elif tipo == "rodovia":
                est["cap_rodovias"] += nivel * 25.0
            elif tipo in industriais or tipo in extrativos:
                est["uso"] += nivel * 2.0
                est["urbanizacao_predios"] += nivel * 2.0 if tipo in industriais else nivel * 1.0
            elif tipo in agricolas:
                est["uso"] += nivel * 1.0
                est["urbanizacao_predios"] += nivel * 0.5
            elif tipo in governo_militar:
                est["uso"] += nivel * 1.0
                est["urbanizacao_predios"] += nivel * 2.0

# Calcular capacidades totais
paises_stats = defaultdict(lambda: {"uso": 0.0, "cap": 0.0, "estados": 0, "com_gargalo": 0})

for eid, est in estados.items():
    urbanizacao = est["urbanizacao_predios"] + est["pop_urbana"] / 25000.0
    cap_urb = min(40.0, urbanizacao * 0.10)
    base_terreno = 20.0 + (5.0 if est["eh_costeiro"] else 0.0)
    cap_total = base_terreno + cap_urb + est["cap_portos"] + est["cap_ferrovias"] + est["cap_rodovias"]
    est["cap_total"] = cap_total
    
    pid = est["pais_id"]
    st = paises_stats[pid]
    st["uso"] += est["uso"]
    st["cap"] += cap_total
    st["estados"] += 1
    if est["uso"] > cap_total:
        st["com_gargalo"] += 1

print("\n--- Estatísticas Atuais por País (Sem Ferrovias/Rodovias) ---")
for pid in ["usa", "gbr", "fra", "bel", "deu", "sov", "bra", "nga", "cog", "ang", "ind"]:
    st = paises_stats[pid]
    saldo = st["cap"] - st["uso"]
    pct = (st["cap"] / st["uso"] * 100) if st["uso"] > 0 else 100
    print(f"País {pid.upper():<5}: Estados={st['estados']:<3} | Cap={st['cap']:<10.0f} | Uso={st['uso']:<10.0f} | Saldo={saldo:<+10.0f} ({pct:5.1f}%) | Estados com Gargalo={st['com_gargalo']}/{st['estados']}")
