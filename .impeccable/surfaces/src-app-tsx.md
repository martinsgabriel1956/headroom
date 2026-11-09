---
version: 1
slug: "src-app-tsx"
primary_target: "src/App.tsx"
related_targets: []
---

# Main screen (scan + clean)

Scope: the single main window of the Headroom desktop app. Mode: Operate.
Task: see where regenerable artifacts take space, select, confirm, delete. Occasional use, when the disk is tight.
Confirmed with user: the disk map leads; the list is grouped by project; it must not read as antivirus, generic dashboard, toy, or terminal costume.
Unresolved: scanning-progress and empty states get real data once the Rust backend exists; the UI uses labeled sample data until then.

## Direction contract

THESIS: The disk drawn as an architectural floor plan. Each project is a room whose area is its real recoverable space, partitioned into alcoves per artifact, with each area given in GB as a dimension. The plan refuses the category default of a rainbow sunburst or treemap beside a table.

OWN-WORLD: Drafting-paper off-white ground, black ink walls (thick for project rooms, hairline partitions for artifacts), graphite annotations, and one drafting blue that only means "selected": diagonal hatch on the plan and a filled checkbox in the list. Active projects are drawn in poché (fine horizontal rule fill) with an ATIVO stamp. Type is Barlow throughout with tabular figures; room names are set like plan lettering. No cards, no shadows, no gradients.

STORY: The user sees at a glance which rooms are big and what is inside them. They select alcoves on the plan or rows in the list, which mirror each other. They read the exact real GB in the dimension line, confirm, and get the space back.

FIRST VIEWPORT: A top bar with the wordmark, scan root, Real/Aparente segmented toggle and Escanear button. Below it, the plan at full width, about 45% of the height. Then the grouped list (checkbox, artifact, rule, modified, size). A footer holds the dimension line, which measures the selected portion against the rest, and the primary "Limpar N GB" button at the right.

FORM: Floor plan with dimensioned rooms. Position 1 on my ordered list (IMPECCABLE’S PICK). seed:a22348b0e0bb6046

Signature interaction: hovering or selecting an alcove highlights its list row and the reverse; the footer dimension line re-measures live as the selection changes.

FINISH: unreviewed and undocumented is unfinished; this build ends with the finish review, the verdict, DESIGN.md, and every shipping raster carrying its provenance
