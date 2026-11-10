import subprocess
import re
from collections import defaultdict

proc = subprocess.run(['git', '-C', '/home/rafael/claude/hege-wt/a-distribuicao', 'show', 'jogavel:assets/data/economia/provincias.ron'], capture_output=True, text=True)

provs = []
for line in proc.stdout.splitlines():
    if not line.strip() or line.strip().startswith('//'):
        continue
    m_pais = re.search(r'pais_id:"([^"]+)"', line)
    if not m_pais or m_pais.group(1) != 'usa':
        continue
    m_id = re.search(r'id:"([^"]+)"', line)
    m_nome = re.search(r'nome:"([^"]+)"', line)
    m_est = re.search(r'estado_id:"([^"]+)"', line)
    m_terra = re.search(r'terra_aravel_total:([0-9]+)', line)
    m_infra = re.search(r'infraestrutura_base:([0-9.]+)', line)
    
    predios = []
    for m in re.finditer(r'tipo_predio_id:"([^"]+)"[^)]*?nivel:([0-9]+)', line):
        predios.append({
            'tipo': m.group(1),
            'nivel': int(m.group(2))
        })
    
    recursos = {}
    m_rec = re.search(r'limites_recursos:\{([^}]+)\}', line)
    if m_rec:
        for rm in re.finditer(r'"([^"]+)":([0-9]+)', m_rec.group(1)):
            recursos[rm.group(1)] = int(rm.group(2))
            
    provs.append({
        'id': m_id.group(1),
        'nome': m_nome.group(1),
        'estado_id': m_est.group(1),
        'terra': int(m_terra.group(1)) if m_terra else 0,
        'infra': float(m_infra.group(1)) if m_infra else 35.0,
        'predios': predios,
        'recursos': recursos,
    })

cur_pop_by_id = {}
with open('/home/rafael/claude/hege-wt/a-distribuicao/assets/data/economia/provincias.ron') as f:
    for line in f:
        if 'pais_id:"usa"' in line:
            m_id = re.search(r'id:"([^"]+)"', line)
            tot_p = sum(float(m.group(1)) for m in re.finditer(r'populacao:([0-9.]+)', line))
            cur_pop_by_id[m_id.group(1)] = tot_p

tot_levels_jog = defaultdict(int)
for p in provs:
    for b in p['predios']:
        tot_levels_jog[b['tipo']] += b['nivel']

vagas_ratio = {
    'administracao_governamental': 485.0, 'campo_gas_natural': 355.0, 'complexo_militar_quartel': 495.0,
    'estaleiro_naval': 555.0, 'fabrica_ferramentas': 430.0, 'fabrica_maquinas': 470.0,
    'fabrica_textil': 455.0, 'fazenda_graos': 515.0, 'industria_alimenticia': 420.0,
    'mina_bauxita': 400.0, 'mina_carvao': 500.0, 'mina_cobre': 440.0, 'mina_ferro': 460.0,
    'mina_uranio': 405.0, 'poco_petroleo': 420.0, 'porto_comercial': 430.0, 'porto_pesca': 335.0,
    'serraria': 400.0, 'setor_servicos_comercio': 430.0, 'siderurgica': 480.0, 'usina_termeletrica': 385.0
}

rec_map = {
    'mina_ferro': 'ferro', 'mina_carvao': 'carvao', 'poco_petroleo': 'petroleo',
    'campo_gas_natural': 'gas_natural', 'mina_cobre': 'cobre', 'mina_uranio': 'uranio',
    'mina_bauxita': 'bauxita'
}

for p in provs:
    if p['nome'] == 'Guam' or p['id'] in ('gum_00', 'asm_5000'):
        p['predios'] = [b for b in p['predios'] if b['tipo'] != 'mina_ferro']
    if p['nome'] == 'Iowa' or p['estado_id'] == 'usa_iowa':
        p['predios'] = [b for b in p['predios'] if b['tipo'] != 'mina_ferro']
    if p['nome'] == 'Nova Iorque' or p['estado_id'] == 'usa_nova_iorque':
        p['predios'] = [b for b in p['predios'] if b['tipo'] != 'poco_petroleo']

prov_levels = {p['id']: defaultdict(int) for p in provs}

for tipo, tot_nacional in tot_levels_jog.items():
    eligible = [p for p in provs if any(b['tipo'] == tipo for b in p['predios'])]
    if not eligible:
        continue
    
    for p in eligible:
        prov_levels[p['id']][tipo] = 1
    rem = tot_nacional - len(eligible)
    if rem <= 0:
        continue
        
    weights = {}
    if tipo in rec_map:
        rec = rec_map[tipo]
        for p in eligible:
            weights[p['id']] = float(p['recursos'].get(rec, 0))
        if sum(weights.values()) == 0:
            for p in eligible:
                weights[p['id']] = cur_pop_by_id.get(p['id'], 1.0)
    elif tipo.startswith('fazenda_') or tipo.startswith('plantacao_') or tipo.startswith('rancho_'):
        for p in eligible:
            weights[p['id']] = float(p['terra'])
    else:
        # Industrial / Services / Government / Porto Pesca / Serraria
        # Weighted by pop * infra
        for p in eligible:
            pop = cur_pop_by_id.get(p['id'], 1.0)
            weights[p['id']] = pop * p['infra']
            
    soma_w = sum(weights.values())
    quotas = {}
    floors = {}
    remainders = {}
    for p in eligible:
        q = (rem * weights[p['id']]) / soma_w
        fl = int(q)
        quotas[p['id']] = q
        floors[p['id']] = fl
        remainders[p['id']] = q - fl
        prov_levels[p['id']][tipo] += fl
        
    leftover = rem - sum(floors.values())
    sorted_rem = sorted(eligible, key=lambda p: remainders[p['id']], reverse=True)
    for i in range(leftover):
        prov_levels[sorted_rem[i]['id']][tipo] += 1

print("Preservation check:")
for tipo, tot in tot_levels_jog.items():
    s = sum(prov_levels[p['id']][tipo] for p in provs)
    assert s == tot

print("Iowa fazenda_graos:", prov_levels['usa_3541']['fazenda_graos'])
print("Minnesota iron:", prov_levels['usa_3526']['mina_ferro'], "Iowa iron:", prov_levels['usa_3541']['mina_ferro'])
print("Texas oil:", prov_levels['usa_3548']['poco_petroleo'], "NY oil:", prov_levels['usa_3559']['poco_petroleo'])
print("Guam iron:", prov_levels.get('gum_00', {}).get('mina_ferro', 0))
print("Tutuila pesca:", prov_levels['asm_4998']['porto_pesca'])
print("Massachusetts fazenda:", prov_levels['usa_3525']['fazenda_graos'])
