"""One-period provision and contribution study. Run directly to reproduce the cases."""
from itertools import product

from care_continuity import share


def majority(members, ballots):
    """Absolute majority of the whole assembly, including abstainers in the denominator."""
    if not members or not ballots.keys() <= members or any(type(v) is not bool for v in ballots.values()):
        raise ValueError("invalid assembly or ballot")
    return sum(ballots.values()) * 2 > len(members)


def select(scores, threshold, places):
    """An unresolved cutoff tie leaves those positions open for a further round."""
    eligible = sorted((s for s in scores.values() if s >= threshold), reverse=True)
    if places < 1 or not eligible:
        return set()
    cutoff = eligible[min(places, len(eligible)) - 1]
    tied = len(eligible) > places and eligible[places] == cutoff
    return {person for person, score in scores.items()
            if score > cutoff or (score == cutoff and not tied)}


def allocate(needs, treasury, capacity, approved, awards):
    """Homogeneous service units, one settlement unit each, capacity net of reserves."""
    if any(type(n) is not int or n < 0 for n in [*needs.values(), treasury, capacity]):
        raise ValueError("invalid quantity")
    base = share(needs, min(treasury, capacity))
    used = sum(base.values())
    claims = dict.fromkeys(needs, 0)
    seen = set()
    for event, person, quantity, rate, reviewers in awards:
        if event in seen or person not in needs or person in reviewers or len(set(reviewers)) < 2:
            raise ValueError("invalid contribution evidence")
        if any(type(n) is not int or n < 0 for n in (quantity, rate)):
            raise ValueError("invalid contribution quantity or rate")
        seen.add(event)
        claims[person] += quantity * rate
    limit = min(treasury, capacity) - used if approved else 0
    extra = share(claims, limit)
    spending = used + sum(extra.values())
    return base, extra, treasury - spending, capacity - spending


def replay_commitments(opening, needs, fixed, capacities, discretionary, delay):
    """Reserve every future firm payment before distributing any optional awards."""
    n = len(needs)
    if not n or any(len(series) != n for series in (fixed, capacities, discretionary)):
        raise ValueError("inconsistent horizon")
    if any(type(v) is not int or v < 0
           for v in [opening, delay, *needs, *fixed, *capacities, *discretionary]):
        raise ValueError("expected unsigned integer quantities")
    firm = [base + wage for base, wage in zip(needs, fixed)]
    windows = [sum(firm[max(0, t-delay):t+1]) for t in range(n)]
    minimum = max(windows)
    if any(f > q for f, q in zip(firm, capacities)) or opening < minimum:
        raise ValueError("firm schedule cannot be offered")
    residual = [opening - w for w in windows]
    awards = []
    for t in range(n):
        stop = min(n, t + delay + 1)
        award = min(discretionary[t], capacities[t] - firm[t], min(residual[t:stop]))
        awards.append(award)
        for j in range(t, stop):
            residual[j] -= award
    # Independent replay. Payments clear after the allocation in their due period.
    free = opening
    pending = {}
    for t, (obligation, award) in enumerate(zip(firm, awards)):
        paid = obligation + award
        assert paid <= min(free, capacities[t])
        free -= paid
        assert free == residual[t]
        pending[t + delay] = paid
        free += pending.pop(t, 0)
        assert free + sum(pending.values()) == opening
    return minimum, awards, free, sum(pending.values())


def verify_small_horizons():
    """Exhaustively compare earliest-first awards with feasible payment schedules."""
    for fixed in product(range(2), repeat=3):
        for delay, opening in product(range(4), range(2, 9)):
            feasible = []
            for awards in product(range(3), repeat=3):
                free = opening
                paid = []
                for t, award in enumerate(awards):
                    charge = 1 + fixed[t] + award
                    if charge > min(free, 3):
                        break
                    paid.append(charge)
                    free -= charge
                    if t >= delay:
                        free += paid[t-delay]
                else:
                    feasible.append(awards)
            try:
                _, actual, _, _ = replay_commitments(opening, [1]*3, fixed, [3]*3, [2]*3, delay)
            except ValueError:
                assert not feasible
            else:
                assert tuple(actual) == max(feasible)


def multi_period_study():
    periods = 60
    base = [440] * periods
    fixed = [48, 60] * 30
    capacity = [520] * periods
    optional = [12] * periods
    for delay in (0, 1, 2):
        low = max(sum(base[j] + fixed[j] for j in range(max(0, t-delay), t+1))
                  for t in range(periods))
        result = replay_commitments(low, base, fixed, capacity, optional, delay)
        print("delay", delay, "minimum opening", low, "optional paid", sum(result[1]))
        try:
            replay_commitments(low-1, base, fixed, capacity, optional, delay)
        except ValueError:
            pass
        else:
            raise AssertionError("underfunded schedule accepted")
    # At t=0, isolated accounting suggests 12 units can be paid. At t=1 all
    # 988 units are needed for overlapping firm payments, so none are available.
    assert replay_commitments(988, base, fixed, capacity, optional, 1)[1] == [0]*periods
    assert replay_commitments(1000, base, fixed, capacity, optional, 1)[1] == [12, 0]*30
    assert replay_commitments(1000, base, fixed, capacity, [6]*periods, 1)[1] == [6]*periods
    assert replay_commitments(1012, base, fixed, capacity, optional, 1)[1] == optional
    try:
        replay_commitments(2000, base, fixed, [487] + capacity[1:], optional, 1)
    except ValueError:
        return
    raise AssertionError("physical shortfall concealed by treasury")


def study():
    needs = dict(engineer=110, operator=100, learner=90, resident=140)
    voters = set(needs)
    approval = majority(voters, dict(engineer=True, learner=True, resident=True))
    assert approval and not majority(voters, dict(engineer=True, learner=True))
    candidates = dict(engineer=91, operator=83)
    selected = select(candidates, 80, 1)
    assert selected == {"engineer"}
    assert select(dict(engineer=91, operator=91), 80, 1) == set()
    # Two different roles. Selection for the specialist vacancy earns no points by itself.
    reviews = {"peer", "recipient"}
    awards = [
        *((f"specialist-shift-{person}", person, 8, 4, reviews) for person in sorted(selected)),
        ("service-shift", "operator", 8, 2, reviews),
        ("school-portfolio", "learner", 1, 12, reviews),
    ]
    for label, cash, output, vote, expected in [
        ("approved", 520, 500, approval, [32, 16, 12, 0]),
        ("capacity constrained", 520, 470, approval, [16, 8, 6, 0]),
        ("liquidity constrained", 470, 520, approval, [16, 8, 6, 0]),
        ("base shortage", 520, 430, approval, [0, 0, 0, 0]),
        ("no mandate", 520, 500, False, [0, 0, 0, 0]),
    ]:
        base, extra, free, spare = allocate(needs, cash, output, vote, awards)
        assert list(extra.values()) == expected
        assert base == allocate(needs, cash, output, vote, [])[0]
        spent = sum(base.values()) + sum(extra.values())
        assert spent + free == cash and spent + spare == output
        print(label, "base", sum(base.values()), "awards", extra, "cash remaining", free)
    # Offer fixed-rate work only when both the base and the whole work commitment fit.
    work, school = awards[:-1], awards[-1:]
    for cash, output, can_offer in [(500, 500, True), (500, 490, True),
                                    (500, 487, False), (487, 500, False)]:
        commitment = sum(quantity * rate for _, _, quantity, rate, _ in work)
        feasible = approval and min(cash, output) >= sum(needs.values()) + commitment
        assert feasible == can_offer
        if feasible:
            base, wages, free, spare = allocate(needs, cash, output, approval, work)
            assert sum(wages.values()) == commitment and base == needs
            _, prizes, _, _ = allocate(dict.fromkeys(needs, 0), free, spare, approval, school)
            assert prizes["learner"] == min(12, min(cash, output) - 488)
    for invalid in (awards + awards[:1], [("x", "engineer", 1, 4, {"engineer", "peer"})]):
        try:
            allocate(needs, 520, 500, True, invalid)
        except ValueError:
            continue
        raise AssertionError("invalid evidence accepted")


if __name__ == "__main__":
    study()
    multi_period_study()
    verify_small_horizons()
