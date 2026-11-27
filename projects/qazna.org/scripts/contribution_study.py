"""One-period provision and contribution study. Run directly to reproduce the cases."""
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
