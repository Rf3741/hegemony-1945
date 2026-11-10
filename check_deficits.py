import proto_sim5

# Check if any province has vagas < desired_emp
deficit_provs = []
for p in proto_sim5.provs:
    pop_p = proto_sim5.cur_pop_by_id[p['id']]
    vagas_p = sum(proto_sim5.prov_levels[p['id']][t] * proto_sim5.vagas_ratio.get(t, 450.0) for t in proto_sim5.prov_levels[p['id']])
    work_pop_p = pop_p * (114113958.0 / 140000000.0)
    desired_emp_p = work_pop_p * 0.8696
    if vagas_p < desired_emp_p:
        deficit_provs.append((p['nome'], pop_p, desired_emp_p, vagas_p, desired_emp_p - vagas_p))

print(f"Total provinces with deficit: {len(deficit_provs)}")
for name, pop, des, vag, diff in sorted(deficit_provs, key=lambda x: x[4], reverse=True)[:10]:
    print(f"  {name:25} pop={pop:10.0f} des={des:10.0f} vag={vag:10.0f} deficit={diff:10.0f}")
