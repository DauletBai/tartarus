#!/usr/bin/env python3
"""Illustrative closed-community resource and circulation audit; not a forecast."""
from decimal import Decimal as D


def transfer(accounts, sender, receiver, amount):
    amount = D(str(amount))
    if amount < 0 or accounts[sender] < amount:
        raise ValueError('Недостаточно средств или отрицательное перечисление')
    accounts[sender] -= amount
    accounts[receiver] += amount


def calculate(energy_produced=D('1000')):
    population = D('100')
    food_per_person = D('8')
    energy_per_person = D('5')
    food_output = D('1000')
    food_energy = D('200')
    food_price = D('3')
    energy_price = D('2')
    if energy_produced < food_energy:
        raise ValueError('Для заданного выпуска пищи не хватает энергии')
    food_need = population * food_per_person
    energy_need = population * energy_per_person
    food_delivered = min(food_need, food_output)
    energy_delivered = min(energy_need, energy_produced - food_energy)
    accounts = {'люди': D('2000'), 'производства': D('3000'), 'Казна': D('5000')}
    initial = sum(accounts.values())
    transfer(accounts, 'Казна', 'люди', 2000)
    transfer(accounts, 'производства', 'люди', 1000)
    transfer(accounts, 'люди', 'производства', food_delivered * food_price)
    transfer(accounts, 'люди', 'производства', energy_delivered * energy_price)
    transfer(accounts, 'производства', 'Казна', 2000)
    assert sum(accounts.values()) == initial
    assert food_delivered <= food_output
    assert food_energy + energy_delivered <= energy_produced
    return {'energy_produced':energy_produced,'food_need':food_need,
            'food_delivered':food_delivered,'energy_need':energy_need,
            'energy_delivered':energy_delivered,'accounts':accounts,'total':initial}


def circulation_period(accounts, saving):
    # Demonstration with public production and no taxes, credit or money creation.
    saving = D(str(saving))
    pulse = D('1000')
    wages = D('600')
    if not D('0') <= saving <= pulse - wages:
        raise ValueError('Недопустимое накопление для заданного порядка выплат')
    if accounts['Казна'] < pulse:
        return False
    initial = sum(accounts.values())
    transfer(accounts, 'Казна', 'люди', pulse)
    transfer(accounts, 'люди', 'производства', pulse - saving)
    transfer(accounts, 'производства', 'люди', wages)
    transfer(accounts, 'люди', 'производства', wages)
    proceeds = accounts['производства']
    transfer(accounts, 'производства', 'Казна', proceeds)
    assert sum(accounts.values()) == initial
    return True


def daily_needs_cycle():
    # Complete-fulfilment example, not a policy for unspent allocations.
    accounts = {'Казна': D('35'), 'человек A': D('0'),
                'человек B': D('0'), 'производства': D('0')}
    initial_total = sum(accounts.values())
    for day, requests in enumerate(((D('10'), D('20')), (D('10'), D('25'))), 1):
        required = sum(requests)
        if accounts['Казна'] < required:
            raise ValueError('Недостаточно существующей меры для следующего дня')
        for person, amount in zip(('человек A', 'человек B'), requests):
            transfer(accounts, 'Казна', person, amount)
            transfer(accounts, person, 'производства', amount)
        transfer(accounts, 'производства', 'Казна', required)
        assert sum(accounts.values()) == initial_total
        assert accounts['человек A'] == accounts['человек B'] == D('0')
        assert accounts['Казна'] == initial_total
        print('День', day, ': потребности', requests,
              '; текущая мера Казны', accounts['Казна'], '; новая эмиссия 0')
    return accounts


def cohort_capacity_example():
    # People aged a at year t belong to the cohort born in year t-a.
    births = {year: 1000 for year in range(-20, 4)}
    ages = {'детский сад': range(3, 6), 'школа': range(6, 17),
            'образование после школы — возможный максимум': range(17, 21)}
    def demand(year, age_range):
        return sum(births[year - age] for age in age_range)
    base = {name: demand(3, bounds) for name, bounds in ages.items()}
    assert base == {'детский сад': 3000, 'школа': 11000,
                    'образование после школы — возможный максимум': 4000}
    planned_places = (base['детский сад'] * 105 + 99) // 100
    births[0] = 1200
    actual_need = demand(3, ages['детский сад'])
    reserve_target = (actual_need * 105 + 99) // 100
    assert planned_places == 3150 and actual_need == 3200 and reserve_target == 3360
    print('Поколения: базовые потребности', base, '; план мест сада с резервом 5%:', planned_places)
    print('Сценарий: +200 рождений в году 0, в году 3 детей в возрасте 3–5 лет:',
          actual_need, '; недостача до покрытия:', max(0, actual_need - planned_places),
          '; до цели с резервом:', max(0, reserve_target - planned_places))
    return base, actual_need, planned_places, reserve_target


def education_choice_example():
    # Single-period illustration: one person makes one choice, places are not interchangeable.
    choices = {'искусство': 60, 'наука': 40}
    places = {'искусство': 50, 'наука': 50}
    shortage = {field: max(0, choices[field] - places[field]) for field in choices}
    unused = {field: max(0, places[field] - choices[field]) for field in choices}
    assert sum(choices.values()) == sum(places.values()) == 100
    assert shortage == {'искусство': 10, 'наука': 0}
    assert unused == {'искусство': 0, 'наука': 10}
    print('Свободный выбор: желающих', choices, '; мест', places,
          '; нехватка', shortage, '; свободные места', unused)
    return choices, places, shortage, unused


def fixed_basket_audit(food_supply, energy_supply):
    """One closed accounting period; no prediction or production cost model."""
    # Reference basket: 2 kg food + 3 kWh = 10 accounting units.
    # Both prices are a public convention, not a consequence of the basket alone.
    prices = {'пища': D('2'), 'энергия': D('2')}
    basket = {'пища': D('2'), 'энергия': D('3')}
    people = {'A': {'пища': D('2'), 'энергия': D('2')},
              'B': {'пища': D('3'), 'энергия': D('3')}}
    available = {'пища': D(str(food_supply)), 'энергия': D(str(energy_supply))}
    assert sum(basket[g] * prices[g] for g in basket) == 10
    demand = {g: sum(person[g] for person in people.values()) for g in basket}
    shortages = {g: max(D('0'), demand[g] - available[g]) for g in basket}
    allocation = {name: sum(prices[g] * qty for g, qty in needs.items())
                  for name, needs in people.items()}
    accounts = {'Казна': D('20'), 'люди': D('0'), 'производства': D('0')}
    transfer(accounts, 'Казна', 'люди', sum(allocation.values()))
    # Only actual goods are paid for; distribution in a shortage is not specified.
    sold = {g: min(demand[g], available[g]) for g in demand}
    payment = sum(sold[g] * prices[g] for g in sold)
    transfer(accounts, 'люди', 'производства', payment)
    transfer(accounts, 'производства', 'Казна', payment)
    assert sum(accounts.values()) == 20
    assert allocation == {'A': D('8'), 'B': D('12')}
    return demand, shortages, allocation, accounts


def automation_economy_example(food_capacity, energy_capacity, automated):
    """Two goods, one time period, physical production and a closed monetary cycle."""
    population = D('100')
    need_food, need_energy = population, population
    energy_for_food = D('0.2')
    ai_energy = D('10') if automated else D('0')
    total_energy_required = need_energy + energy_for_food * need_food + ai_energy
    food_gap = max(D('0'), need_food - D(str(food_capacity)))
    energy_gap = max(D('0'), total_energy_required - D(str(energy_capacity)))
    labour = D('55') if automated else D('80')
    planned_payment = 2 * need_food + need_energy
    accounts = {'Казна': D('300'), 'люди': D('0'), 'производства': D('0')}
    if not food_gap and not energy_gap:
        transfer(accounts, 'Казна', 'люди', planned_payment)
        transfer(accounts, 'люди', 'производства', planned_payment)
        transfer(accounts, 'производства', 'Казна', planned_payment)
    # When infeasible, no invented allocation or payment rule is applied.
    assert sum(accounts.values()) == D('300')
    return {'продовольствие': need_food, 'энергия необходимая': total_energy_required,
            'энергия для ИИ': ai_energy, 'дефицит пищи': food_gap,
            'дефицит энергии': energy_gap, 'трудочасы': labour,
            'расчётный поток': planned_payment, 'счета': accounts}


def universal_and_extra_example():
    # Three people have equal basic provision; two choose additional benefits.
    needs = {'A': D('10'), 'B': D('10'), 'C': D('10')}
    extra = {'A': D('0'), 'B': D('3'), 'C': D('2')}
    # One unit of measure buys one unit of a single illustrative good.
    output = D('35')
    allocations = {name: needs[name] + extra[name] for name in needs}
    assert sum(allocations.values()) == output
    accounts = {'Казна': D('35'), 'люди': D('0'), 'производства': D('0')}
    transfer(accounts, 'Казна', 'люди', sum(allocations.values()))
    transfer(accounts, 'люди', 'производства', sum(allocations.values()))
    transfer(accounts, 'производства', 'Казна', sum(allocations.values()))
    assert accounts['Казна'] == 35 and sum(accounts.values()) == 35
    return needs, extra, allocations, output, accounts


def multi_year_state(energy_shock_year=None):
    """Three-year fixed plan with stocks, capacity turnover and timed treasury flows."""
    accounts = {'Казна': D('400'), 'люди': D('0'), 'производства': D('0')}
    capacity = {'пища': D('110'), 'энергия': D('150'), 'лечение': D('12')}
    mineral_stock = D('100')
    records = []
    for year in range(3):
        pop = D(100 + 5 * year)
        food = pop + 5  # including five optional extra units
        care = pop / 10  # direct, not purchased by people
        energy = pop + D('.2') * food + D('.5') * care + 10 + 4
        hours = D('.2') * food + D('.1') * energy + 2 * care + 10
        mineral_use = D('9')  # 5 for operations; 4 for repair and expansion
        actual_energy_capacity = capacity['энергия'] - (D('20') if year == energy_shock_year else 0)
        failures = []
        for name, required, maximum in [('пища', food, capacity['пища']),
                                        ('энергия', energy, actual_energy_capacity),
                                        ('лечение', care, capacity['лечение']),
                                        ('часы', hours, D('80')),
                                        ('минерал', mineral_use, mineral_stock)]:
            if required > maximum:
                failures.append((name, required - maximum))
        cash = 2 * food + pop
        direct = D('50')  # care 20, maintenance and expansion 30, stipulated sums
        if cash > accounts['Казна'] or direct > accounts['Казна']:
            failures.append(('касса', max(cash, direct) - accounts['Казна']))
        record = {'год': year + 1, 'жителей': pop, 'пища': food, 'энергия': energy,
                  'лечение': care, 'часы': hours, 'минерал до': mineral_stock,
                  'выплата людям': cash, 'прямые услуги': direct,
                  'мощности до': dict(capacity), 'сбои': failures}
        if failures:
            records.append(record)
            break  # no allocation or fabricated outcomes for an infeasible period
        transfer(accounts, 'Казна', 'люди', cash)
        transfer(accounts, 'люди', 'производства', cash)
        transfer(accounts, 'производства', 'Казна', cash)
        transfer(accounts, 'Казна', 'производства', direct)
        transfer(accounts, 'производства', 'Казна', direct)
        mineral_stock -= mineral_use
        capacity['пища'] += 10 - 5
        capacity['энергия'] += 10 - 5
        capacity['лечение'] += D('1') - D('.5')
        assert sum(accounts.values()) == 400 and min(accounts.values()) >= 0
        assert mineral_stock >= 0
        record['минерал после'] = mineral_stock
        record['мощности после'] = dict(capacity)
        record['счета'] = dict(accounts)
        records.append(record)
    return records


def solve_linear(matrix, rhs):
    """Gaussian elimination, with pivoting; used only for the 4x4 technology system."""
    n = len(rhs)
    augmented = [list(map(D, matrix[i])) + [D(rhs[i])] for i in range(n)]
    for k in range(n):
        pivot = max(range(k, n), key=lambda i: abs(augmented[i][k]))
        if abs(augmented[pivot][k]) < D('1e-20'):
            raise ValueError('Сингулярная матрица')
        augmented[k], augmented[pivot] = augmented[pivot], augmented[k]
        divisor = augmented[k][k]
        augmented[k] = [v / divisor for v in augmented[k]]
        for i in range(n):
            if i != k:
                factor = augmented[i][k]
                augmented[i] = [a - factor * b for a, b in zip(augmented[i], augmented[k])]
    return [row[-1] for row in augmented]


def five_year_state(energy_loss=D('0'), demand_factor=D('1')):
    # Rows = input, columns = output. All quantities in million standard bundles/year.
    a = [[D('0'), D('0'), D('0'), D('0')],
         [D('.2'), D('0'), D('.2'), D('.15')],
         [D('0'), D('0'), D('0'), D('0')],
         [D('.1'), D('.08'), D('.05'), D('.1')]]
    identity_minus_a = [[D(int(i == j)) - a[i][j] for j in range(4)] for i in range(4)]
    initial_capacity = [D('140'), D('200'), D('120'), D('110')]
    growth = [D('1.05'), D('1.05'), D('1.03'), D('1.04')]
    labour = [D('.07'), D('.04'), D('.10'), D('.08')]
    mineral_stock = D('1000')
    treasury = D('500')  # millions of units, simple within-year settlement
    rows = []
    for t in range(5):
        population = D('100') * D('1.01') ** t
        # Additional 10 industrial bundles are a fixed construction program.
        demand = [population * demand_factor, D('1.2') * population * demand_factor,
                  population * demand_factor, D('.4') * population * demand_factor + 10]
        output = solve_linear(identity_minus_a, demand)
        residuals = [output[i] - sum(a[i][j] * output[j] for j in range(4)) - demand[i] for i in range(4)]
        capacity = [initial_capacity[i] * growth[i] ** t for i in range(4)]
        capacity[1] -= energy_loss
        hours = sum(labour[i] * output[i] for i in range(4))
        extraction = D('.05') * output[1] + D('.25') * output[3]
        mineral_stock -= extraction
        household_cash = 2 * demand[0] + demand[1] + 3 * D('.2') * population * demand_factor
        direct = demand[2] + 3 * (D('.2') * population * demand_factor + 10)
        feasible = (all(output[i] <= capacity[i] for i in range(4)) and hours <= 80
                    and extraction <= 80 and mineral_stock >= 700
                    and max(household_cash, direct) <= treasury)
        assert max(map(abs, residuals)) < D('1e-20')
        rows.append({'год': t+1, 'население млн': population, 'выпуск': output,
                     'мощности': capacity, 'труд млн условных дней': hours,
                     'добыча': extraction, 'запас': mineral_stock,
                     'начисление людям млн': household_cash, 'прямые услуги млн': direct,
                     'баланс max': max(map(abs, residuals)), 'выполнимо': feasible})
    return rows


def demographic_five_year_state(birth_rate=D('.018')):
    """Synthetic annual cohorts + 10-sector physical balance; quantities in millions."""
    groups = ['food', 'energy', 'water', 'housing', 'health', 'education',
              'transport', 'industry', 'digital', 'care']
    # Illustrative initial cohorts: 25% age 0–14, 65% age 15–64, 10% age 65–100.
    cohorts = [D('25')/15]*15 + [D('65')/50]*50 + [D('10')/36]*36
    death_weight = [D('1.5') if i == 0 else D('.3') if i < 15 else D('.6') if i < 65 else D('6') for i in range(101)]
    # Rows are input sectors, columns are output sectors; coefficients synthetic.
    a = [[D('0') for j in groups] for i in groups]
    for row, values in {1: ['.08','0','.08','.08','.04','.03','.08','.08','.15','.02'],
                        2: ['.05','.01','0','.02','.02','.01','.01','.02','.01','.01'],
                        7: ['.06','.05','.04','.12','.05','.03','.08','.08','.10','.02'],
                        8: ['.01','.02','.01','.01','.04','.04','.02','.02','.02','.02'],
                        6: ['.04','.01','.01','.05','.01','.01','0','.02','.01','.01']}.items():
        a[row] = list(map(D, values))
    matrix = [[D(int(i == j)) - a[i][j] for j in range(10)] for i in range(10)]
    mineral = D('2500')
    rows = []
    for t in range(5):
        total = sum(cohorts)
        pre = sum(cohorts[3:6])
        pupils = sum(cohorts[6:18])
        students = sum(cohorts[18:23]) * D('.55')
        older = sum(cohorts[65:])
        adults = sum(cohorts[18:65])
        # Enrolment is a scenario, not an assumption that schooling determines entitlement.
        teachers = pre/D('20') + pupils/D('23') + students/D('15')
        health_staff = total * D('49')/D('10000')
        housing_growth = max(D('0'), (total - D('100')) / D('3'))
        # 0.5m renewals per year plus provision for additional households;
        # accumulated need converted to annual net new requirement below.
        previous_growth = rows[-1]['housing_growth'] if rows else D('0')
        homes = D('.5') + housing_growth - previous_growth
        # Sector units: annual per-person service bundles, housing=million homes.
        final = [total, total*D('1.2'), total, homes, total,
                 pre+pupils+students, total, total*D('.6')+D('10'),
                 total, older*D('.5')]
        output = solve_linear(matrix, final)
        residual = [output[i] - sum(a[i][j]*output[j] for j in range(10)) - final[i] for i in range(10)]
        extraction = output[7]*D('.20') + output[1]*D('.05')
        mineral -= extraction
        assert mineral >= 0 and max(map(abs, residual)) < D('1e-20')
        # Planned financial amounts are illustrative accounting allocations, not cost estimates.
        prices = list(map(D, ['200','100','40','1000','180','150','120','130','60','120']))
        budget = [final[i]*prices[i] for i in range(10)]  # million Tenge
        adult_headcount = adults
        births = total * birth_rate
        denominator = sum(cohorts[i]*death_weight[i] for i in range(101))
        death_rate_multiplier = total*D('.008')/denominator
        survivors = [cohorts[i]*(1-death_weight[i]*death_rate_multiplier) for i in range(101)]
        next_cohorts = [births] + survivors[:100]
        next_cohorts[100] += survivors[100]
        deaths = total - sum(survivors)
        assert abs(sum(next_cohorts) - (total + births - deaths)) < D('1e-20')
        rows.append({'year':t+1, 'population':total, 'births':births, 'deaths':deaths,
                     'preschool':pre,'pupils':pupils,'students':students,'older':older,
                     'adults':adults,'teachers':teachers,'health_staff':health_staff,
                     'homes':homes,'housing_growth':housing_growth,'output':output,'final':final,
                     'labour_estimate':teachers+health_staff,'mineral_stock':mineral,
                     'budget':budget,'budget_total':sum(budget),'max_residual':max(map(abs,residual))})
        cohorts = next_cohorts
    return rows


def illustrative_double_entry():
    # A mathematical proposal; Treasury spending is not supported by Qazna's base Transfer.
    accounts = {'эмиссионная запись': D('0'), 'Казна': D('0'),
                'люди': D('0'), 'производства': D('0')}
    accounts['эмиссионная запись'] -= D('100')
    accounts['Казна'] += D('100')
    transfer(accounts, 'Казна', 'люди', 40)
    transfer(accounts, 'люди', 'производства', 30)
    transfer(accounts, 'производства', 'Казна', 30)
    assert sum(accounts.values()) == 0
    circulation = sum(value for key, value in accounts.items() if key != 'эмиссионная запись')
    assert circulation == 100 and accounts['эмиссионная запись'] == -100
    return accounts, circulation


def main():
    demographic = demographic_five_year_state()
    assert len(demographic) == 5 and abs(demographic[0]['population']-100)<D('1e-20')
    assert all(r['births'] > r['deaths'] and r['teachers'] < r['adults'] for r in demographic)
    for i in range(4):
        assert abs(demographic[i+1]['population'] - (demographic[i]['population'] +
                   demographic[i]['births'] - demographic[i]['deaths'])) < D('1e-20')
    assert all(sum(r['budget']) == r['budget_total'] for r in demographic)
    low_birth = demographic_five_year_state(birth_rate=D('.015'))
    high_birth = demographic_five_year_state(birth_rate=D('.021'))
    assert low_birth[-1]['population'] < demographic[-1]['population'] < high_birth[-1]['population']
    print('Проверка рождаемости 15 / 18 / 21 на 1000: население к году 5, млн:',
          low_birth[-1]['population'], demographic[-1]['population'], high_birth[-1]['population'])
    for row in demographic:
        print('Демографический план 100 млн:', row)
    five = five_year_state()
    assert all(row['выполнимо'] for row in five)
    stress = five_year_state(energy_loss=D('50'))
    assert not stress[0]['выполнимо']
    for row in five:
        print('Государство 100 млн:', row)
    print('Проверка снижения мощности энергии на 50:', [r['выполнимо'] for r in stress])
    baseline = multi_year_state()
    assert len(baseline) == 3 and not any(row['сбои'] for row in baseline)
    assert baseline[-1]['минерал после'] == 73
    assert baseline[0]['энергия'] == 140 and baseline[2]['энергия'] == D('152.5')
    shock = multi_year_state(energy_shock_year=1)
    assert len(shock) == 2 and shock[-1]['сбои']
    for row in baseline:
        print('Трёхлетний план:', row)
    print('Контроль сбоя второго года:', shock[-1])
    print('Гарантия и дополнительное признание:', universal_and_extra_example())
    normal = automation_economy_example(100, 135, False)
    automated = automation_economy_example(100, 135, True)
    shock = automation_economy_example(100, 115, True)
    assert normal['энергия необходимая'] == 120 and normal['трудочасы'] == 80
    assert automated['энергия необходимая'] == 130 and automated['трудочасы'] == 55
    assert shock['дефицит энергии'] == 15
    for label, result in [('без ИИ', normal), ('с ИИ', automated), ('сбой энергии', shock)]:
        print('Связанная экономика:', label, result)
    for label, food, energy in [('эталонный период', 5, 5), ('сбой энергии', 5, 4)]:
        demand, shortage, allocation, balances = fixed_basket_audit(food, energy)
        print('Постоянная корзина:', label, '; потребность', demand,
              '; необеспечено', shortage, '; выделено', allocation, '; счета', balances)
        assert sum(balances.values()) == 20
        if energy == 4:
            assert shortage['энергия'] == 1 and balances['люди'] == 2
        else:
            assert not any(shortage.values()) and balances['Казна'] == 20
    cohort_capacity_example()
    education_choice_example()
    daily_needs_cycle()
    cyclic = {'Казна': D('1000'), 'люди': D('0'), 'производства': D('0')}
    assert circulation_period(cyclic, 0) and cyclic == {'Казна': D('1000'), 'люди': D('0'), 'производства': D('0')}
    assert circulation_period(cyclic, 100) and cyclic == {'Казна': D('900'), 'люди': D('100'), 'производства': D('0')}
    before = dict(cyclic)
    assert not circulation_period(cyclic, 0) and cyclic == before
    print('Два круга без эмиссии: после сбережения остатки', cyclic,
          '; следующий пульс 1000 недоступен из остатка Казны 900')
    ledger, circulation = illustrative_double_entry()
    print('Условная двойная запись:', ledger, '; мера на счетах =', circulation, '; сумма всех записей =', sum(ledger.values()))
    for name, amount in [('обычный период', 1000), ('энергетический сбой', 400)]:
        r = calculate(D(amount))
        print(name, 'пища', r['food_delivered'], '/', r['food_need'],
              'энергия', r['energy_delivered'], '/', r['energy_need'],
              'остатки', r['accounts'], 'всего', r['total'])
    initial = D('30')
    growth_target = initial * D('1.04')
    slowing_target = initial * D('2') / D('1.8')
    assert growth_target - initial == D('1.20')
    assert slowing_target > initial
    print('M̂ при росте 4% и постоянной скорости:', growth_target,
          '; при нулевом росте и снижении V с 2 до 1,8:', slowing_target)
    print('Условие сценария: отдельные взаимные расчёты за промежуточную энергию и ресурсы не моделируются.')


def extended_book_calculations():
    """Reproducible synthetic LP + engineering envelopes; not calibrated forecasts."""
    import numpy as np
    import scipy
    from scipy.optimize import linprog
    import math
    sectors = ['food','energy','water','housing','health','education',
               'transport','industry','digital','care']
    demographics = demographic_five_year_state()
    n, T = 10, 5
    A = np.zeros((n,n))
    for i, vals in {1:[.08,0,.08,.08,.04,.03,.08,.08,.15,.02],
                    2:[.05,.01,0,.02,.02,.01,.01,.02,.01,.01],
                    7:[.06,.05,.04,.12,.05,.03,.08,.08,.10,.02],
                    8:[.01,.02,.01,.01,.04,.04,.02,.02,.02,.02],
                    6:[.04,.01,.01,.05,.01,.01,0,.02,.01,.01]}.items(): A[i]=vals
    f = np.array([[float(v) for v in r['final']] for r in demographics])
    f[:,7] -= 10  # Replace the previous exogenous construction allowance with B*b.
    B = np.diag(np.full(n,.05))
    B[7,:] += [.25,.5,.25,1.0,.4,.4,.3,.25,.35,.3]
    B[1,:] += .05
    delta = np.full(n,.02)
    prices = np.array([200,100,40,1000,180,150,120,130,60,120.])
    labour = np.array([.03,.01,.015,4.,.02,.055,.025,.08,.015,.35])
    build_labour = np.array([.01,.02,.01,1.,.02,.02,.01,.02,.02,.02])
    workforce = np.array([float(r['adults']-r['students'])*.72 for r in demographics])
    raw = np.zeros(n); raw[1]=.05;raw[7]=.2
    x_reference = np.linalg.solve(np.eye(n)-A,f[0])
    K_unprepared = x_reference*1.08
    K0 = K_unprepared.copy()
    K0[3] = 1.0  # Explicit pre-plan housing capacity:1m homes/year vs0.54m.
    # 50 x, 50 expansion b, 60 K, one shared demand-coverage scalar.
    NV = 161
    ix=lambda t,j:t*n+j
    ib=lambda t,j:50+t*n+j
    ik=lambda t,j:100+t*n+j
    lam=160
    exposure = np.array([.15,.12,.10,.10,.30,.40,.35,.40,.75,.08])
    ai_configs = {
        'gradual': {'adoption':[.10,.18,.25,.32,.40], 'saving':.5, 'energy':.01, 'digital':.05, 'equipment':.01},
        'accelerated': {'adoption':[.15,.30,.45,.60,.75], 'saving':.7, 'energy':.025, 'digital':.10, 'equipment':.02},
        'rapid': {'adoption':[.20,.40,.60,.80,.95], 'saving':.85, 'energy':.05, 'digital':.20, 'equipment':.04},
    }
    def solve_case(demand_factor=1.,energy_capacity=1.,workforce_factor=1.,reserve=1.05,prepared=True,ai=None,supply_shock=False):
        demand=f*demand_factor
        # Conditional AI resource requirements: NOT empirical forecasts.
        extra=np.zeros_like(demand)
        effective_labour=np.tile(labour,(T,1))
        effective_build_labour=np.tile(build_labour,(T,1))
        if ai is not None:
            for t in range(T):
                adoption=ai['adoption'][t]
                reduction=exposure*adoption*ai['saving']
                effective_labour[t]*=(1-reduction)
                effective_build_labour[t]*=(1-reduction[7])
                ramp=adoption/ai['adoption'][-1]
                extra[t,1]=x_reference[1]*ai['energy']*ramp
                extra[t,8]=f[t,8]*ai['digital']*ramp
                extra[t,7]=f[t,7]*ai['equipment']*ramp
        availability=np.ones((T,n))
        if supply_shock:
            availability[2:,1]=.8
            availability[2:,7]=.9
        initial=(K0 if prepared else K_unprepared).copy();initial[1]*=energy_capacity
        eq=[];beq=[];ub=[];bub=[]
        for t in range(T):
            for i in range(n):
                row=np.zeros(NV)
                for j in range(n):
                    row[ix(t,j)]=float(i==j)-A[i,j]
                    row[ib(t,j)]=-B[i,j]
                row[lam]=-demand[t,i];eq.append(row);beq.append(extra[t,i])
                row=np.zeros(NV);row[ik(t+1,i)]=1;row[ik(t,i)]=-(1-delta[i]);row[ib(t,i)]=-1
                eq.append(row);beq.append(0.)
                row=np.zeros(NV);row[ix(t,i)]=1;row[ik(t,i)]=-availability[t,i];ub.append(row);bub.append(0.)
            row=np.zeros(NV)
            for j in range(n):row[ix(t,j)]=effective_labour[t,j];row[ib(t,j)]=effective_build_labour[t,j]
            ub.append(row);bub.append(workforce[t]*workforce_factor)
            row=np.zeros(NV)
            for j in range(n):row[ix(t,j)]=raw[j]
            ub.append(row);bub.append(50.)
        for j in range(n):
            row=np.zeros(NV);row[ik(0,j)]=1;eq.append(row);beq.append(initial[j])
            row=np.zeros(NV);row[ix(4,j)]=reserve;row[ik(5,j)]=-1;ub.append(row);bub.append(0.)
        row=np.zeros(NV)
        for t in range(T):
            for j in range(n):row[ix(t,j)]=raw[j]
        ub.append(row);bub.append(200.)  # Initial mineral stock2500, floor2300.
        bounds=[(0,None)]*NV
        for t in range(T):
            for j in range(n):bounds[ib(t,j)]=(0,float((K0 if prepared else K_unprepared)[j]*.2))
        bounds[lam]=(1,1)
        c=np.zeros(NV)
        for t in range(T):
            for j in range(n):c[ix(t,j)]=prices[j]
        E=np.array(eq);e=np.array(beq);U=np.array(ub);u=np.array(bub)
        strict=linprog(c,A_ub=U,b_ub=u,A_eq=E,b_eq=e,bounds=bounds,method='highs')
        coverage=1.
        if not strict.success:
            assert strict.status==2, strict.message
            bounds[lam]=(0,1)
            diag=np.zeros(NV);diag[lam]=-1
            test=linprog(diag,A_ub=U,b_ub=u,A_eq=E,b_eq=e,bounds=bounds,method='highs')
            assert test.success,test.message
            coverage=float(test.x[lam]);bounds[lam]=(coverage,coverage)
            result=linprog(c,A_ub=U,b_ub=u,A_eq=E,b_eq=e,bounds=bounds,method='highs')
        else: result=strict
        assert result.success,result.message
        v=result.x;x=v[:50].reshape(T,n);b=v[50:100].reshape(T,n);k=v[100:160].reshape(T+1,n)
        residual=float(np.max(np.abs(E@v-e)));violation=float(max(0,np.max(U@v-u)))
        assert residual<1e-6 and violation<1e-6
        assert np.min(v)>-1e-7 and np.all(x<=k[:-1]+1e-6)
        years=[]
        for t in range(T):
            construction=B@b[t]
            inter=A@x[t]
            final=demand[t]*coverage
            assert np.max(abs(x[t]-inter-final-construction-extra[t]))<1e-6
            years.append({'year':t+1,'output':x[t].tolist(),'capacity':k[t].tolist(),
                          'expansion':b[t].tolist(),'construction_demand':construction.tolist(),
                          'final':final.tolist(),'labour_million_fte':float(effective_labour[t]@x[t]+effective_build_labour[t]@b[t]),
                          'ai_resource_demand':extra[t].tolist(),
                          'ai_resource_value_million':float(prices@extra[t]),
                          'effective_labour_coefficients':effective_labour[t].tolist(),
                          'gross_task_saving_million_fte':float((labour-effective_labour[t])@x[t]+(build_labour-effective_build_labour[t])@b[t]),
                          'workforce_limit_million':float(workforce[t]*workforce_factor),
                          'extraction':float(raw@x[t]),'final_value_million':float(prices@final),
                          'construction_value_million':float(prices@construction),
                          'gross_value_million':float(prices@x[t]),
                          'intermediate_value_million':float(prices@inter)})
        return {'strict_feasible':bool(strict.success),'proportional_coverage':coverage,
                'max_balance_residual':residual,'max_constraint_violation':violation,
                'variables':NV,'equalities':len(eq),'inequalities':len(ub),
                'years':years,'end_capacity':k[-1].tolist(),
                'mineral_end':2500-sum(y['extraction'] for y in years)}
    cases={
        'unprepared_housing_capacity':solve_case(prepared=False),
        'base':solve_case(),
        'demand_plus_10_percent':solve_case(demand_factor=1.1),
        'energy_capacity_minus_20_percent':solve_case(energy_capacity=.8),
        'workforce_minus_50_percent':solve_case(workforce_factor=.5),
        'terminal_reserve_15_percent':solve_case(reserve=1.15)}
    assert cases['base']['strict_feasible']
    assert not cases['unprepared_housing_capacity']['strict_feasible']
    assert cases['energy_capacity_minus_20_percent']['proportional_coverage']<1
    ai_results={name:solve_case(ai=config) for name,config in ai_configs.items()}
    ai_results['accelerated_supply_shock']=solve_case(ai=ai_configs['accelerated'],supply_shock=True)
    for name,case in ai_results.items():
        for t,y in enumerate(case['years']):
            y['calendar_year']=2027+t
            if case['strict_feasible']:
                base_labour=cases['base']['years'][t]['labour_million_fte']
                y['net_released_million_fte']=base_labour-y['labour_million_fte']
                y['equivalent_week_if_baseline_40h']=40*y['labour_million_fte']/base_labour
        case['five_year_ai_resource_value_million']=sum(y['ai_resource_value_million'] for y in case['years'])
        case['five_year_total_final_value_million']=sum(y['final_value_million']+y['construction_value_million']+y['ai_resource_value_million'] for y in case['years'])
    assert all(ai_results[n]['strict_feasible'] for n in ai_configs)
    assert not ai_results['accelerated_supply_shock']['strict_feasible']
    for case in ai_results.values():
        for y in case['years']:
            assert abs(y['gross_value_million']-y['intermediate_value_million']
                       -y['final_value_million']-y['construction_value_million']
                       -y['ai_resource_value_million'])<1e-5
    facilities=[]
    for r in demographics:
        N=float(r['population'])*1e6;pre=float(r['preschool'])*1e6;pupils=float(r['pupils'])*1e6
        facilities.append({'year':r['year'],'population':N,
            'births':float(r['births'])*1e6,'deaths':float(r['deaths'])*1e6,
            'preschool_places_5percent_reserve':math.ceil(pre*1.05),
            'preschool_buildings_200places':math.ceil(pre*1.05/200),
            'school_places_5percent_reserve':math.ceil(pupils*1.05),
            'school_buildings_1000places':math.ceil(pupils*1.05/1000),
            'preschool_staff_fte_ratio20_cover1_25':pre/20*1.25,
            'school_teacher_fte_ratio23':pupils/23,
            'hospital_beds_admission0_15_los6_occupancy0_8':math.ceil(N*.15*6/(365*.8)),
            'outpatient_doctor_fte_6visits_1800visits_per_year':N*6/1800,
            'homes_construction':float(r['homes'])*1e6})
    infrastructure=[]
    for N in (100_000_000,1_000_000_000,8_000_000_000):
        for ops_daily in (2,6,12):
            avg=N*ops_daily/86400;peak=10*avg
            raw_tb=N*ops_daily*3*2000/1e12
            # Conditional node throughput (not a benchmark):20k durable ops/s at50%target.
            logical=math.ceil(peak/(20000*.5));physical=logical*3
            hot=raw_tb*90*3
            cold=raw_tb*(5*365-90)*.3*1.5
            storage=hot+cold
            capex=physical*20000+storage*70
            it_kw=physical*.4+storage*.002
            facility_kw=it_kw*1.4
            electricity=facility_kw*8760
            # Dollar amounts are freely chosen inputs, NOT market quotes or Tenge exchange rates.
            opex=200*120000+capex*.15+electricity*.1
            infrastructure.append({'population':N,'ops_per_person_day':ops_daily,
                'average_ops_s':avg,'peak_ops_s':peak,'raw_events_tb_day':raw_tb,
                'logical_nodes_assuming20k_ops_s':logical,'physical_nodes_3replicas':physical,
                'hot_storage_tb':hot,'cold_storage_tb':cold,'total_storage_tb':storage,
                'conditional_hardware_usd':capex,'facility_kw':facility_kw,
                'electricity_kwh_year':electricity,'conditional_annual_opex_usd':opex,
                'conditional_five_year_tco_usd':capex+5*opex,
                'logical_nodes_if5k_ops_s':math.ceil(peak/(5000*.5)),
                'logical_nodes_if50k_ops_s':math.ceil(peak/(50000*.5))})
    circulation=[]
    first=cases['base']['years'][0]
    annual_final_flow=(first['final_value_million']+first['construction_value_million'])*1e6
    daily_flow=annual_final_flow/365
    for cycle_days in (.25,1,3):
        circulation.append({'cycle_days':cycle_days,'daily_final_flow_tenge':daily_flow,
             'annual_final_flow_tenge':daily_flow*365,
             'circulating_stock_with20percent_buffer':daily_flow*cycle_days*1.2})
    result={'status':'synthetic scenario, not a calibrated national plan or hardware benchmark',
            'versions':{'numpy':np.__version__,'scipy':scipy.__version__},
            'sectors':sectors,'A':A.tolist(),'B':B.tolist(),'depreciation':delta.tolist(),
            'accounting_weights':prices.tolist(),'labour_coefficients':labour.tolist(),
            'construction_labour_coefficients':build_labour.tolist(),'initial_capacity':K0.tolist(),'unprepared_initial_capacity':K_unprepared.tolist(),
            'capacity_expansion_cap_fraction':.2,'mineral_initial':2500,'mineral_floor':2300,
            'assumptions':{'mortality_per_1000':8,'births_per_1000':18,'migration':0,
            'accounting_weights_status':'arbitrary constant weights; not market prices or a calibrated Tenge standard',
            'infrastructure_status':'hypothetical input costs and throughput; excludes buildings, WAN, GPU, sector systems, user devices and implementation transitions',
            'population_at_year_start':True},
            'cases':cases,'facilities':facilities,'infrastructure':infrastructure,
            'ai_transition':{'as_of':'2026-11-01','horizon':'2027-2031',
                'status':'conditional scenarios, not employment or war forecasts',
                'sector_task_exposure_assumptions':exposure.tolist(),
                'configs':ai_configs,'cases':ai_results,
                'shock_assumptions':'From 2029 energy capacity availability 0.8 and industry 0.9; no probabilities assigned',
                'limits':'Same population, A/B, prices and basic needs as base. Extra AI energy, digital services and equipment are scenario bundles, not TWh/GPU measurements. No skill matching, retraining cost, wage distribution, initial unemployment or military operations model.'},
            'circulation':circulation,
            'end_of_year5_population_million':float(demographics[-1]['population']+demographics[-1]['births']-demographics[-1]['deaths'])}
    return result


def write_extended_book_calculations():
    import json
    from pathlib import Path
    result=extended_book_calculations()
    path=Path(__file__).resolve().parents[1]/'tenge_calculations.json'
    path.write_text(json.dumps(result,ensure_ascii=False,indent=2)+'\n',encoding='utf-8')
    print('Saved',path)
    for name,r in result['cases'].items():
        print(name,'strict=',r['strict_feasible'],'coverage=',round(r['proportional_coverage'],6),
              'residual=',r['max_balance_residual'],'mineral_end=',r['mineral_end'])
    for name,r in result['ai_transition']['cases'].items():
        print('AI',name,'strict=',r['strict_feasible'],
              'coverage=',round(r['proportional_coverage'],6),
              'year5_labour_million=',round(r['years'][-1]['labour_million_fte'],6),
              'residual=',r['max_balance_residual'])
    return result


if __name__ == '__main__':
    import sys
    if '--extended' in sys.argv:
        write_extended_book_calculations()
    else:
        main()
