# Diagram-Design Research Report

- **Source:** [github.com/cathrynlavery/diagram-design](https://github.com/cathrynlavery/diagram-design) | [diagramdesign.dev](https://diagramdesign.dev)
- **Author:** Cathryn Lavery
- **License:** Open Source
- **Category:** Editorial-Grade Diagram Architecture & Multi-Agent Skill
- **Analyzed for:** AeroEdits AI Agent & Motion Graphics Architecture

---

## 1. Executive Summary

**Diagram-Design** (`cathrynlavery/diagram-design`) is an open-source agent skill created by designer and entrepreneur Cathryn Lavery. It is designed to replace ugly, amateurish AI diagrams (often derided as "Mermaid slop"—cluttered boxes, harsh drop shadows, awkward line crossings, and neon colors) with **editorial-grade, publication-ready diagrams**.

Compatible with Claude Code, Cursor, Copilot, and Codex, Diagram-Design can generate **45 distinct structural diagram types** as self-contained, responsive `.html` and `.svg` files. It includes automated brand extraction from URLs and can upgrade existing Mermaid or Excalidraw files into clean, professional visual assets.

---

## 2. Core Architecture & Diagram Catalog

```
[Prompt: "Explain our Kafka to ClickHouse real-time data streaming pipeline"]
                                 │
                                 ▼
                    [1. Diagram Classification]
         (Selects 1 of 45 structural patterns: e.g. Data Flow Pipeline)
                                 │
                                 ▼
                    [2. Brand & Style Ingestion]
         (Pulls palette & typography from target URL or project style)
                                 │
                                 ▼
                    [3. Editorial Design Rules]
         • Strict typographic hierarchy (optical sizing, line tracking)
         • Orthogonal routing (no overlapping or chaotic diagonal lines)
         • Clean whitespace grouping (no harsh borders or generic shadows)
                                 │
                                 ▼
                    [4. Self-Contained Output]
         • Scalable Vector Graphics (.svg) + semantic `.html`
```

### 2.1 The 45 Diagram Typology (Key Categories)

| Category | Diagram Archetypes | Explainer Use Case in Tutorials |
| :--- | :--- | :--- |
| **System & Tech Architecture** | Cloud infrastructure, microservices mesh, event-driven pipelines, API gateways. | *"Here is how our microservices communicate over gRPC."* |
| **Process & Flow** | Decision trees, user journey funnels, multi-lane swimlane flows, state machines. | *"When a user signs up, here is the verification sequence."* |
| **Strategic & Analytical** | 2x2 quadrants, Venn diagrams, flywheel/circular cycles, pyramid hierarchies. | *"Comparing latency vs. throughput across 4 database engines."* |
| **Relational & Data** | Entity-Relationship (ERD), schema trees, AST node graphs, network topologies. | *"The relational structure between users, workspaces, and permissions."* |
| **Temporal & Chronological** | Multi-phase roadmaps, milestone timelines, Git commit branch graphs. | *"The evolution of this protocol from v1 to v3."* |

---

## 3. Deep Dive: Key Technical Capabilities

### 3.1 The "Anti-Mermaid" Design Principles
Diagram-Design codifies why typical AI diagrams look bad and enforces mechanical solutions:
1. **No Gratuitous Drop Shadows:** Eliminates muddy, fuzzy black drop shadows (`box-shadow: 0 10px 25px rgba(0,0,0,0.3)`); relies instead on subtle surface contrasts (e.g. 2% tint difference) and 1px crisp borders.
2. **Orthogonal Routing:** Connecting arrows always travel at 90-degree angles with rounded corners (`rx="6"`), strictly avoiding chaotic diagonal cross-overs.
3. **Typography-First Spatial Rhythm:** Label dimensions dictate container size, not vice versa. Generous horizontal and vertical breathing room prevents text clipping.
4. **Color as Meaning, Not Decoration:** Muted neutrals for nodes; bright accent colors reserved strictly for the active flow path or key highlight.

### 3.2 Automated URL Brand Extraction
The skill allows users to provide their product or company URL. It extracts CSS variables, primary hex codes, and font families, immediately skinning the diagram to match the company's identity.

### 3.3 Conversion from Mermaid & Excalidraw
Instead of forcing users to rewrite existing diagrams, Diagram-Design parses Mermaid `.mmd` syntax and Excalidraw JSON files, regenerating them using its editorial layout engine.

---

## 4. AeroEdits vs. Diagram-Design Comparison

| Feature | Diagram-Design | AeroEdits |
| :--- | :--- | :--- |
| **Visual Nature** | Static publication-grade SVGs and HTML pages. | **Animated motion graphics** rendered as video over footage. |
| **Graphic Types** | 45 analytical & architectural diagram types. | Currently focused on metric cards, titles, callouts, and layout splits. |
| **Styling** | Automated URL brand scraper. | Native Style System (`--brand-*` colors, fonts, logo in `styles/`). |
| **Timeline Integration**| Standalone image/SVG export. | Anchored to spoken sentences (`atWord`) on the video timeline. |
| **Animation Awareness**| None (static). | **GSAP 3.15 seekable animations** with custom easing tokens. |

---

## 5. Actionable Takeaways for AeroEdits

### 🟢 What We Should Adopt

1. **Animated "Flow & Architecture" Explainer Templates in HyperFrames:**
   - The biggest gap in tutorial motion graphics is explaining **how systems work** (data flow, client-server handshake, pipeline stages).
   - Adopt Diagram-Design's 5 most popular archetypes as animated HyperFrames templates:
     - **The 3-Stage Pipeline:** (Source $\to$ Transform $\to$ Sink) with an animated glowing pulse traveling along the orthogonal connecting arrow.
     - **The 2x2 Quadrant:** (Four labeled sectors) with animated point placement.
     - **The Architectural Mesh:** (Client $\to$ Gateway $\to$ Microservices $\to$ DB) with sequential node reveals.
2. **Enforce Orthogonal Line Routing & Clean Geometry:**
   - In any graphic involving arrows or connections, ban diagonal lines and fuzzy drop shadows. Enforce clean 90-degree lines with rounded elbows and subtle border contrasts.
3. **SVG `stroke-dashoffset` Flow Animations:**
   - Use GSAP to animate SVG dashed strokes (`stroke-dashoffset: 100` $\to$ `0`) along connecting paths to visually represent real-time data packets or network requests while the speaker discusses API traffic.

### 🔴 What We Refuse / Avoid

1. **Do NOT output static un-animated SVGs on the video timeline:**
   - A video editor is a time-based medium. A completely static diagram that pops up on screen and sits motionless for 8 seconds feels dead and causes viewer drop-off. Every diagram must have an intentional build-in, living motion (pulsing paths or glowing nodes), and an exit.
2. **Do NOT overload the frame with 50-node microservice diagrams:**
   - Publication diagrams in books can have 40 tiny boxes; video graphics cannot. An on-screen graphic must be glanceable within 3–5 seconds. The agent must enforce a maximum limit of 5–7 visual nodes per diagram.
