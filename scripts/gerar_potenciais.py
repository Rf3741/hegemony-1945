#!/usr/bin/env python3
"""
Gera o catálogo de Potencial Regional (assets/data/economia/potenciais.ron)
no estilo Victoria 3 para o jogo Guerra Fria (1945).
"""

import re
import sys
from collections import defaultdict

def main():
    print("Iniciando geração de potenciais regionais...")

    # 1. Carregar estados de assets/data/economia/estados.ron
    estados_eco = {}
    with open("/home/rafael/claude/guerra-fria-jogar/assets/data/economia/estados.ron", "r", encoding="utf-8") as f:
        content = f.read()
    for m in re.finditer(r'\(\s*id:\s*"([^"]+)",\s*nome:\s*"([^"]+)",\s*pais_id:\s*"([^"]+)"', content):
        estados_eco[m.group(1)] = {
            "id": m.group(1),
            "nome": m.group(2),
            "pais_id": m.group(3)
        }
    print(f"1. Carregados {len(estados_eco)} estados de economia.")

    # 2. Carregar províncias de assets/data/economia/provincias.ron para saber prédios iniciais e terra arável
    predios_iniciais_estado = defaultdict(lambda: defaultdict(int))
    terra_aravel_inicial_estado = defaultdict(int)
    recursos_iniciais_estado = defaultdict(lambda: defaultdict(int))

    with open("/home/rafael/claude/guerra-fria-jogar/assets/data/economia/provincias.ron", "r", encoding="utf-8") as f:
        p_content = f.read()

    # Cada província começa com (id:"...", ...
    # Extrai bloco de cada província
    prov_blocks = re.findall(r'\(id:"([^"]+)",nome:"([^"]+)",(?:codigo_externo:Some\("([^"]+)"\),)?estado_id:"([^"]+)",pais_id:"([^"]+)",terra_aravel_total:(\d+),(.*?)\)(?:,\s*\(|\s*\])', p_content, re.DOTALL)
    print(f"2. Analisadas {len(prov_blocks)} províncias.")

    # Mapeamento de prédios para recursos / culturas
    # Rural / Agrícola
    predio_cultura = {
        "fazenda_graos": "graos",
        "rancho_pecuario": "gado",
        "fazenda_hortifruti": "frutas",
        "plantacao_algodao": "algodao",
        "plantacao_acucar": "acucar",
        "plantacao_cafe_cha": "cafe",
        "plantacao_tabaco": "tabaco",
    }
    # Extrativo / Mineral
    predio_recurso = {
        "plantacao_borracha": "borracha",
        "porto_pesca": "pesca",
        "serraria": "madeira",
        "mina_carvao": "carvao",
        "mina_ferro": "ferro",
        "poco_petroleo": "petroleo",
        "campo_gas_natural": "gas_natural",
        "mina_bauxita": "bauxita",
        "mina_cobre": "cobre",
        "mina_chumbo_zinco": "chumbo_zinco",
        "mina_uranio": "uranio",
        "mina_ouro_prata": "ouro_prata",
    }

    for pid, pnome, pcod, eid, pais_id, terra_tot, rest in prov_blocks:
        terra_aravel_inicial_estado[eid] += int(terra_tot)
        
        # Limites recursos da província
        m_lim = re.search(r'limites_recursos:\{([^}]*)\}', rest)
        if m_lim and m_lim.group(1):
            for it in m_lim.group(1).split(","):
                pts = it.split(":")
                if len(pts) == 2:
                    rk = pts[0].strip().strip('"')
                    rv = int(pts[1].strip())
                    recursos_iniciais_estado[eid][rk] += rv

        # Prédios na província
        predios_match = re.findall(r'tipo_predio_id:"([^"]+)",nivel:(\d+)', rest)
        for tpid, nivel_str in predios_match:
            niv = int(nivel_str)
            predios_iniciais_estado[eid][tpid] += niv

    # 3. Carregar staging territorio_dados/estados.ron (clima, terreno, portos, admin1s)
    staging_estados = {}
    with open("/home/rafael/claude/guerra-fria-staging/territorio_dados/estados.ron", "r", encoding="utf-8") as f:
        st_text = f.read()
    st_matches = re.findall(r'"([^"]+)":\s*\((.*?)\),', st_text, re.DOTALL)
    for sid, body in st_matches:
        clima_m = re.search(r'clima:\s*"([^"]+)"', body)
        terreno_m = re.search(r'terreno_predominante:\s*"([^"]+)"', body)
        porto_m = re.search(r'tem_porto:\s*(true|false)', body)
        sub_m = re.search(r'subcontinente:\s*"([^"]+)"', body)
        admin1s = re.findall(r'"([A-Z]{3}-\d+)"', body)
        staging_estados[sid] = {
            "clima": clima_m.group(1) if clima_m else "temperado_continental",
            "terreno": terreno_m.group(1) if terreno_m else "planicie",
            "tem_porto": porto_m.group(1) == "true" if porto_m else False,
            "subcontinente": sub_m.group(1) if sub_m else "europa_ocidental",
            "admin1s": admin1s,
        }
    print(f"3. Carregados {len(staging_estados)} estados de staging.")

    # 4. Carregar recursos canônicos de 1945
    canonico_recursos = {}
    with open("/home/rafael/claude/guerra-fria-staging/provincias_1945/regioes_1945_canonico.ron", "r", encoding="utf-8") as f:
        can_text = f.read()
    can_matches = re.findall(r'"([A-Z]{3}-\d+)":\s*\((.*?)\),', can_text, re.DOTALL)
    for adm1, body in can_matches:
        recs = {}
        m_rec = re.search(r'recursos:\s*\{([^}]*)\}', body)
        if m_rec:
            for it in m_rec.group(1).split(","):
                pts = it.split(":")
                if len(pts) == 2:
                    rk = pts[0].strip().strip('"')
                    recs[rk] = int(pts[1].strip())
        canonico_recursos[adm1] = recs
    print(f"4. Carregadas {len(canonico_recursos)} regiões canônicas de 1945.")

    # 5. Construir potencial para cada estado
    potenciais = []

    for eid, info in sorted(estados_eco.items()):
        nome = info["nome"]
        pais_id = info["pais_id"]
        st = staging_estados.get(eid, {
            "clima": "temperado_continental",
            "terreno": "planicie",
            "tem_porto": False,
            "subcontinente": "europa_ocidental",
            "admin1s": []
        })

        clima = st["clima"]
        terreno = st["terreno"]
        tem_porto = st["tem_porto"]
        subcontinente = st["subcontinente"]

        # 5a. Terra arável agregada
        # Soma da terra arável de canonico para os admin1s ou províncias
        aravel_canonico = sum(canonico_recursos.get(adm, {}).get("terra_aravel", 0) for adm in st["admin1s"])
        aravel_provs = terra_aravel_inicial_estado.get(eid, 0)
        # Níveis agrícolas existentes
        niveis_agricolas_iniciais = sum(
            predios_iniciais_estado[eid].get(p, 0) for p in predio_cultura
        )
        
        # Base de terra arável: no mínimo os níveis existentes + margem (headroom de 20 a 50)
        terra_aravel = max(aravel_canonico, 30, niveis_agricolas_iniciais + 25)
        # Limite razoável para estados grandes
        if aravel_canonico > 0:
            terra_aravel = max(terra_aravel, aravel_canonico + 20)

        # 5b. Culturas permitidas (baseada em clima, geografia e histórico)
        culturas = set()

        # Grãos e Gado são universais em quase todo lugar arável
        if clima not in ("polar_tundra", "polar_glacial", "deserto_frio"):
            culturas.add("graos")
            culturas.add("gado")
            culturas.add("frutas")

        # Clima tropical e subtropical
        if clima in ("tropical_savana", "tropical_moncao", "equatorial", "subtropical_umido"):
            culturas.add("acucar")
            culturas.add("algodao")
            culturas.add("tabaco")

        if clima in ("tropical_savana", "tropical_moncao", "equatorial"):
            culturas.add("cacau")

        # Café: regiões tropicais de planalto/savana/úmido (América Latina, África, Índia, Sudeste Asiático)
        if (clima in ("tropical_savana", "subtropical_umido", "tropical_moncao") and 
            (pais_id in ("bra", "col", "ven", "per", "bol", "ecu", "eth", "ken", "uga", "ang", "moz", "mdg", "ind", "ceyl", "idn", "vnm", "mex", "gtm", "hnd", "slv", "nic", "cri", "pan", "cub", "dom", "hti") or
             subcontinente in ("america_do_sul", "america_central", "caribe", "africa_oriental", "africa_central", "sul_da_asia", "sudeste_asiatico"))):
            culturas.add("cafe")

        # Algodão em climas semi-áridos ou mediterrâneos quentes com irrigação
        if clima in ("semi_arido", "mediterraneo") and pais_id in ("usa", "egy", "sud", "ind", "pak", "uzb", "tkm", "tur", "syr", "irq", "irn", "mex", "bra"):
            culturas.add("algodao")
            culturas.add("tabaco")

        # Beterraba açucareira em clima temperado
        if clima in ("temperado_oceanico", "temperado_continental", "mediterraneo") and pais_id in ("fra", "ger", "pol", "sov", "cze", "aut", "bel", "nld", "usa", "ita", "esp", "gbr"):
            culturas.add("acucar")

        # Tabaco temperado / subtropical
        if pais_id in ("usa", "cub", "bra", "tur", "grc", "bul", "yug", "phl", "idn", "ind", "chn", "ita"):
            culturas.add("tabaco")

        # Borracha natural: equatorial / tropical úmido
        if clima in ("equatorial", "tropical_moncao") or pais_id in ("mys", "idn", "tha", "vnm", "khm", "lao", "cgo", "lbr", "nga", "cmr"):
            culturas.add("borracha")

        # GARANTIA: Qualquer prédio agrícola já presente no estado DEVE ter sua cultura permitida
        for pr_id, cult in predio_cultura.items():
            if predios_iniciais_estado[eid].get(pr_id, 0) > 0:
                culturas.add(cult)

        # 5c. Recursos minerais / extrativos
        recursos = {}

        # Agrega de canonico
        recs_canonico = defaultdict(int)
        for adm in st["admin1s"]:
            for rk, rv in canonico_recursos.get(adm, {}).items():
                if rk != "terra_aravel":
                    recs_canonico[rk] += rv

        for rk, rv in recs_canonico.items():
            recursos[rk] = rv

        # Madeira: regiões com floresta, taiga ou clima temperado/tropical úmido
        if "madeira" not in recursos:
            if clima in ("temperado_oceanico", "temperado_continental", "equatorial", "tropical_moncao") or terreno in ("floresta", "taiga", "montanhas"):
                recursos["madeira"] = 30
            elif clima in ("tropical_savana", "subtropical_umido"):
                recursos["madeira"] = 20

        # Pesca: apenas estados com litoral / acesso ao mar
        if tem_porto or recs_canonico.get("pesca", 0) > 0:
            recursos["pesca"] = max(recursos.get("pesca", 0), 30)
        else:
            # Sem acesso ao mar: pesca = 0
            recursos.pop("pesca", None)

        # GARANTIA: Qualquer prédio extrativo existente no estado DEVE ter cota suficiente
        for pr_id, rec in predio_recurso.items():
            n_existente = predios_iniciais_estado[eid].get(pr_id, 0)
            if n_existente > 0:
                recursos[rec] = max(recursos.get(rec, 0), n_existente + 10)

        # 5d. Qualidade dos Recursos (bônus de produtividade / fertilidade Victoria 3)
        qualidade = {}

        # -------------------------------------------------------------
        # AJUSTES HISTÓRICOS ESPECÍFICOS (Requisitos explícitos da tarefa)
        # -------------------------------------------------------------
        # 1. Minas Gerais: ferro, ouro, café, bauxita
        if eid == "bra_minas_gerais":
            culturas.update(["cafe", "graos", "gado", "acucar", "frutas", "tabaco"])
            recursos["ferro"] = max(recursos.get("ferro", 0), 180)
            recursos["ouro_prata"] = max(recursos.get("ouro_prata", 0), 60)
            recursos["bauxita"] = max(recursos.get("bauxita", 0), 40)
            recursos["madeira"] = max(recursos.get("madeira", 0), 40)
            recursos.pop("pesca", None) # Interior de MG não tem porto marítimo!
            qualidade["cafe"] = 1.25
            qualidade["ferro"] = 1.20
            qualidade["ouro_prata"] = 1.15

        # 2. São Paulo: café (Terra Roxa), açúcar, frutas, porto
        elif eid == "bra_sao_paulo":
            culturas.update(["cafe", "acucar", "graos", "frutas", "gado", "algodao"])
            recursos["pesca"] = max(recursos.get("pesca", 0), 40)
            qualidade["cafe"] = 1.30
            qualidade["acucar"] = 1.15

        # 3. Centro-Oeste: gado e grãos (NÃO café nem cacau)
        elif eid in ("bra_goias", "bra_mato_grosso", "bra_mato_grosso_do_sul", "bra_federal", "bra_tocantins"):
            culturas = {"gado", "graos", "algodao", "frutas"}
            recursos["ouro_prata"] = max(recursos.get("ouro_prata", 0), 25)
            recursos["madeira"] = max(recursos.get("madeira", 0), 30)
            recursos.pop("pesca", None)
            qualidade["gado"] = 1.25
            qualidade["graos"] = 1.15

        # Amazônia brasileira: borracha
        elif eid in ("bra_amazonas", "bra_acre", "bra_para", "bra_rondonia"):
            culturas.update(["borracha", "cacau", "frutas", "graos"])
            recursos["borracha"] = max(recursos.get("borracha", 0), 80)
            recursos["madeira"] = max(recursos.get("madeira", 0), 100)
            qualidade["borracha"] = 1.25

        # Bahia: cacau e tabaco
        elif eid == "bra_bahia":
            culturas.update(["cacau", "tabaco", "acucar", "algodao", "graos", "gado", "frutas"])
            qualidade["cacau"] = 1.30
            qualidade["tabaco"] = 1.20

        # Paraná: café e grãos
        elif eid == "bra_parana":
            culturas.update(["cafe", "graos", "gado", "frutas"])
            qualidade["cafe"] = 1.25

        # 4. Texas: petróleo, gás natural, algodão, gado
        elif eid == "usa_texas":
            culturas.update(["algodao", "gado", "graos", "frutas"])
            recursos["petroleo"] = max(recursos.get("petroleo", 0), 220)
            recursos["gas_natural"] = max(recursos.get("gas_natural", 0), 180)
            recursos["pesca"] = max(recursos.get("pesca", 0), 40)
            qualidade["petroleo"] = 1.25
            qualidade["gas_natural"] = 1.20
            qualidade["algodao"] = 1.15

        # 5. Baku (Absheron / sov_qobustan): petróleo de Baku
        elif eid in ("sov_qobustan", "sov_baku", "aze_baku"):
            recursos["petroleo"] = max(recursos.get("petroleo", 0), 200)
            recursos["gas_natural"] = max(recursos.get("gas_natural", 0), 120)
            qualidade["petroleo"] = 1.30

        # 6. Ruhr (ger_nordrhein_westfalen): carvão e ferro
        elif eid in ("ger_nordrhein_westfalen", "ruhr"):
            recursos["carvao"] = max(recursos.get("carvao", 0), 380)
            recursos["ferro"] = max(recursos.get("ferro", 0), 80)
            qualidade["carvao"] = 1.25

        # 7. Donbass: carvão e ferro
        elif eid in ("donbass", "sov_donbass"):
            recursos["carvao"] = max(recursos.get("carvao", 0), 320)
            recursos["ferro"] = max(recursos.get("ferro", 0), 100)
            qualidade["carvao"] = 1.20
            qualidade["ferro"] = 1.15

        # 8. Katanga (cod_sul): cobre e urânio
        elif eid in ("cod_sul", "cgo_katanga"):
            recursos["cobre"] = max(recursos.get("cobre", 0), 180)
            recursos["uranio"] = max(recursos.get("uranio", 0), 120)
            qualidade["cobre"] = 1.30
            qualidade["uranio"] = 1.25

        # Zâmbia Copperbelt
        elif eid in ("zmb_copperbelt", "zmb_copper"):
            recursos["cobre"] = max(recursos.get("cobre", 0), 160)
            qualidade["cobre"] = 1.25

        # 9. Golfo Pérsico: petróleo e gás natural
        elif eid in ("sau_ash_sharqiyah", "kwt_kuwait", "bhr_bahrein", "qat_catar", "omn_dhofar_sul"):
            recursos["petroleo"] = max(recursos.get("petroleo", 0), 250 if "sau" in eid or "kwt" in eid else 100)
            recursos["gas_natural"] = max(recursos.get("gas_natural", 0), 150)
            qualidade["petroleo"] = 1.30

        # Irã Khuzistão / Abadan
        elif eid in ("irn_isfahan_central", "irn_ahvaz", "irn_khuzestan"):
            recursos["petroleo"] = max(recursos.get("petroleo", 0), 180)
            qualidade["petroleo"] = 1.25

        # Venezuela Lago de Maracaibo
        elif eid in ("ven_zulia_maracaibo",):
            recursos["petroleo"] = max(recursos.get("petroleo", 0), 200)
            qualidade["petroleo"] = 1.25

        # África do Sul Witwatersrand: ouro
        elif eid in ("zaf_transvaal", "zaf_gauteng"):
            recursos["ouro_prata"] = max(recursos.get("ouro_prata", 0), 250)
            recursos["carvao"] = max(recursos.get("carvao", 0), 80)
            qualidade["ouro_prata"] = 1.30

        # Suécia Kiruna: ferro
        elif eid in ("swe_norrbottens", "swe_laponia"):
            recursos["ferro"] = max(recursos.get("ferro", 0), 160)
            qualidade["ferro"] = 1.25

        # Chile Chuquicamata: cobre
        elif eid in ("chl_antofagasta", "chl_atacama"):
            recursos["cobre"] = max(recursos.get("cobre", 0), 200)
            qualidade["cobre"] = 1.25

        # Jamaica: bauxita
        elif eid in ("jam_cornwall", "jam_middlesex", "jam_surrey"):
            recursos["bauxita"] = max(recursos.get("bauxita", 0), 140)
            qualidade["bauxita"] = 1.25

        # Ucrânia Krivoy Rog e Rússia Kursk: ferro
        elif eid in ("sov_dnipropetrovsk", "sov_chernozem_central"):
            recursos["ferro"] = max(recursos.get("ferro", 0), 200)
            qualidade["ferro"] = 1.25

        # Corn Belt americano: grãos
        elif eid in ("usa_iowa", "usa_illinois", "usa_indiana"):
            culturas.update(["graos", "gado"])
            qualidade["graos"] = 1.25

        # Pampas argentinos: gado e grãos
        elif eid in ("arg_buenos_aires", "arg_santa_fe"):
            culturas.update(["gado", "graos"])
            qualidade["gado"] = 1.25
            qualidade["graos"] = 1.20

        # Malaya / Indonésia: borracha
        elif eid in ("mys_malaya", "idn_sumatra", "idn_bornéu", "idn_java"):
            recursos["borracha"] = max(recursos.get("borracha", 0), 160)
            qualidade["borracha"] = 1.25

        # Cuba: açúcar e tabaco
        elif eid in ("cub_oriente", "cub_la_habana", "cub_camaguey"):
            culturas.update(["acucar", "tabaco"])
            qualidade["acucar"] = 1.25
            qualidade["tabaco"] = 1.25

        # Ordenar listas e dicts
        culturas_ordenadas = sorted(culturas)
        recursos_ordenados = {k: v for k, v in sorted(recursos.items())}
        qualidade_ordenada = {k: v for k, v in sorted(qualidade.items())}

        potenciais.append({
            "estado_id": eid,
            "nome": nome,
            "pais_id": pais_id,
            "terra_aravel": terra_aravel,
            "culturas_permitidas": culturas_ordenadas,
            "recursos": recursos_ordenados,
            "qualidade_recursos": qualidade_ordenada,
        })

    print(f"5. Gerados potenciais para {len(potenciais)} estados.")

    # 6. Validação estrita
    erros = []
    for pot in potenciais:
        eid = pot["estado_id"]
        # Validar terra arável >= prédios agrícolas existentes
        n_agri = sum(predios_iniciais_estado[eid].get(p, 0) for p in predio_cultura)
        if pot["terra_aravel"] < n_agri:
            erros.append(f"Estado {eid}: terra arável ({pot['terra_aravel']}) menor que prédios agrícolas existentes ({n_agri})")
        
        # Validar culturas permitidas contém prédios existentes
        for p, cult in predio_cultura.items():
            if predios_iniciais_estado[eid].get(p, 0) > 0 and cult not in pot["culturas_permitidas"]:
                erros.append(f"Estado {eid}: prédio {p} existe mas cultura {cult} não está permitida")

        # Validar recursos minerais contém prédios existentes
        for p, rec in predio_recurso.items():
            n_ext = predios_iniciais_estado[eid].get(p, 0)
            if n_ext > 0:
                lim = pot["recursos"].get(rec, 0)
                if lim < n_ext:
                    erros.append(f"Estado {eid}: prédio {p} nível {n_ext} excede limite de recurso {rec} ({lim})")

    if erros:
        print(f"ERROS DE VALIDAÇÃO ({len(erros)}):")
        for e in erros[:10]:
            print(f"  {e}")
        sys.exit(1)
    else:
        print("Validação concluída: 0 erros!")

    # 7. Escrever assets/data/economia/potenciais.ron
    saida_path = "/home/rafael/claude/guerra-fria-jogar/assets/data/economia/potenciais.ron"
    with open(saida_path, "w", encoding="utf-8") as out:
        out.write("// ==============================================================================\n")
        out.write("// POTENCIAIS REGIONAIS - GUERRA FRIA (1945)\n")
        out.write("// Estilo Victoria 3: limites físicos de expansão, culturas permitidas por clima/solo\n")
        out.write("// e bônus de produtividade por qualidade histórica de jazidas e terras férteis.\n")
        out.write(f"// Total de estados mapeados: {len(potenciais)}\n")
        out.write("// ==============================================================================\n\n")
        out.write("[\n")
        for pot in potenciais:
            out.write("    (\n")
            out.write(f'        estado_id: "{pot["estado_id"]}",\n')
            out.write(f'        terra_aravel: {pot["terra_aravel"]},\n')
            
            # culturas_permitidas
            cults_str = ", ".join(f'"{c}"' for c in pot["culturas_permitidas"])
            out.write(f'        culturas_permitidas: [{cults_str}],\n')
            
            # recursos
            if pot["recursos"]:
                out.write("        recursos: {\n")
                for rk, rv in pot["recursos"].items():
                    out.write(f'            "{rk}": {rv},\n')
                out.write("        },\n")
            else:
                out.write("        recursos: {},\n")

            # qualidade_recursos
            if pot["qualidade_recursos"]:
                out.write("        qualidade_recursos: {\n")
                for qk, qv in pot["qualidade_recursos"].items():
                    out.write(f'            "{qk}": {qv:.2f},\n')
                out.write("        },\n")
            else:
                out.write("        qualidade_recursos: {},\n")

            out.write("    ),\n")
        out.write("]\n")

    print(f"Arquivo gerado com sucesso em: {saida_path}")

if __name__ == "__main__":
    main()
