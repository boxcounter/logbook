---
dimensions:
  - name: Goal
    key: goal
    source: monthly
  - name: Biz
    key: biz
    source: static
    values:
      - Product
      - Marketing
      - Engineering
commitments:
  - role: Dev
    allocation: 40
    goals:
      - Ship it
      - Review
  - role: PM
    allocation: 10
    goals:
      - Planning
---
