import test_redist

tot_pop = 0.0
tot_work_pop = 0.0
tot_desemp = 0.0
tot_vagas = 0.0

print(f"{'Provincia':25} {'Pop':>10} {'Vagas':>10} {'Emp':>10} {'Unemp%':>8}")
for p in sorted(test_redist.provs, key=lambda x: test_redist.cur_pop_by_id.get(x['id'], 0), reverse=True)[:15]:
    pop_p = test_redist.cur_pop_by_id[p['id']]
    vagas_p = sum(test_redist.prov_levels[p['id']][t] * test_redist.vagas_ratio.get(t, 450.0) for t in test_redist.prov_levels[p['id']])
    work_pop_p = pop_p * (114113958.0 / 140000000.0)
    desired_emp_p = work_pop_p * 0.8696
    actual_emp_p = min(desired_emp_p, vagas_p)
    desemp_p = work_pop_p - actual_emp_p
    unemp_rate = desemp_p / work_pop_p if work_pop_p > 0 else 0
    print(f"{p['nome']:25} {pop_p:10.0f} {vagas_p:10.0f} {actual_emp_p:10.0f} {unemp_rate*100:7.1f}%")

for p in test_redist.provs:
    pop_p = test_redist.cur_pop_by_id[p['id']]
    vagas_p = sum(test_redist.prov_levels[p['id']][t] * test_redist.vagas_ratio.get(t, 450.0) for t in test_redist.prov_levels[p['id']])
    work_pop_p = pop_p * (114113958.0 / 140000000.0)
    desired_emp_p = work_pop_p * 0.8696
    actual_emp_p = min(desired_emp_p, vagas_p)
    desemp_p = work_pop_p - actual_emp_p
    tot_pop += pop_p
    tot_work_pop += work_pop_p
    tot_desemp += desemp_p
    tot_vagas += vagas_p

print(f"\nUSA TOTALS:")
print(f"Total Desemprego (if clamped to vagas): {tot_desemp / tot_work_pop * 100:.2f}%")
print(f"Total Desemprego (if not clamped, original): 13.04%")
