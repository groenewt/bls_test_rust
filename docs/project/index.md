# Rusty BLS Documentation Home

![Rusty BLS Processing Banner](../resources/subheader_02.png)

Welcome to the Rusty BLS Data Processing documentation hub. Use this page to quickly navigate to component documentation, roadmaps, and test specifications.

## Visual Site Map

```mermaid
graph LR
    A[Docs Home] --> C[Configuration]
    A --> D[Data]
    A --> P[Processing]
    A --> O[Output]
    A --> E[Error]
    A --> U[Utils]
    A --> PL[Plugin]

    C --- CTasks[Config Tasks]
    D --- DTasks[Data Tasks]
    P --- PTasks[Processing Tasks]
    O --- OTasks[Output Tasks]
    E --- ETasks[Error Tasks]
    PL --- PLTasks[Plugin Tasks]
    U --- UTasks[Utils Tasks]

    click C "components/config/index.md" "Open Configuration Docs"
    click D "components/data/index.md" "Open Data Docs"
    click P "components/processing/index.md" "Open Processing Docs"
    click O "components/output/index.md" "Open Output Docs"
    click E "components/error/index.md" "Open Error Docs"
    click U "components/utils/index.md" "Open Utils Docs"
    click PL "components/plugin/index.md" "Open Plugin Docs"

    click CTasks "components/config/tasks.md" "Open Config Tasks"
    click DTasks "components/data/tasks.md" "Open Data Tasks"
    click PTasks "components/processing/tasks.md" "Open Processing Tasks"
    click OTasks "components/output/tasks.md" "Open Output Tasks"
    click ETasks "components/error/tasks.md" "Open Error Tasks"
    click PLTasks "components/plugin/tasks.md" "Open Plugin Tasks"
    click UTasks "components/utils/tasks.md" "Open Utils Tasks"
```

## Quick Navigation
- Component Index: components/index.md
- System Overview: README.md

## What to Read First
- New to Rusty? Start with README.md for a full architecture overview
- Want to see progress? Open any component’s tasks page
- Running tests? Check each component’s test_specifications.md

---

### Shortcuts
- Configuration: components/config/index.md | components/config/tasks.md
- Data: components/data/index.md | components/data/tasks.md
- Processing: components/processing/index.md | components/processing/tasks.md
- Output: components/output/index.md | components/output/tasks.md
- Error: components/error/index.md | components/error/tasks.md
- Plugin: components/plugin/index.md | components/plugin/tasks.md
- Utils: components/utils/index.md | components/utils/tasks.md
