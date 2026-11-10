#!/usr/bin/env python3
import re
import json

# Read the RON file
with open('/home/rafael/claude/hege-wt/a-distribuicao/assets/data/economia/provincias.ron', 'r') as f:
    content = f.read()

# Find all provinces with pais_id:"chi"
# Pattern to match each province entry
province_pattern = r'\(id:"([^"]+)",nome:"([^"]+)",[^)]*pais_id:"chi"[^)]*pops:\[([^\]]+)\]'

# Actually, let's use a simpler approach - find all lines with pais_id:"chi"
# and extract the pops array for each

# Split by province entries (each starts with (id:)
provinces = re.split(r'\n\s*\(', content)

results = []
total_pop = 0

for prov in provinces:
    if 'pais_id:"chi"' in prov:
        # Extract id and nome
        id_match = re.search(r'id:"([^"]+)"', prov)
        nome_match = re.search(r'nome:"([^"]+)"', prov)
        
        if id_match and nome_match:
            prov_id = id_match.group(1)
            prov_nome = nome_match.group(1)
            
            # Find all populacao values in this province
            pop_values = re.findall(r'populacao:([\d.]+)', prov)
            prov_total = sum(float(p) for p in pop_values)
            
            results.append({
                'id': prov_id,
                'nome': prov_nome,
                'populacao_total': prov_total
            })
            total_pop += prov_total

# Sort by population descending
results.sort(key=lambda x: x['populacao_total'], reverse=True)

print(f"Total provinces with pais_id:'chi': {len(results)}")
print(f"Total population: {total_pop:,.2f}")
print("\nFirst 10 entries (by population):")
for i, r in enumerate(results[:10]):
    print(f"  {i+1}. {r['nome']} ({r['id']}): {r['populacao_total']:,.2f}")

print("\nAll entries:")
for i, r in enumerate(results):
    print(f"  {i+1}. {r['nome']} ({r['id']}): {r['populacao_total']:,.2f}")