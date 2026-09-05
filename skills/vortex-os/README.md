# VORTEX-OS

> **The Native Autonomous Studio Command Center**
> *Stop managing tasks. Start commanding a native digital workforce.*

---

## Table of Contents

1. [What Is VORTEX-OS?](#1-what-is-vortex-os)
2. [Why Native MiniMax?](#2-why-native-minimax)
3. [The 4-Tier Chain of Command](#3-the-4-tier-chain-of-command)
4. [The 6 Architecture Pillars](#4-the-6-architecture-pillars)
5. [Quick Start](#5-quick-start)
6. [200 Use Cases — What Can You Build?](#6-200-use-cases--what-can-you-build)
7. [The Complete Command Reference](#7-the-complete-command-reference)
8. [How To Submit A Master Objective](#8-how-to-submit-a-master-objective)
9. [How HITL Works](#9-how-hitl-works)
10. [How The Continuity Engine Works](#10-how-the-continuity-engine-works)
11. [How The Self-Healing Optimizer Works](#11-how-the-self-healing-optimizer-works)
12. [How To Add Your Own Worker Agent](#12-how-to-add-your-own-worker-agent)
13. [How To Save & Reuse Workflows (Golden Path)](#13-how-to-save--reuse-workflows-golden-path)
14. [The 8 Contract Invariants](#14-the-8-contract-invariants)
15. [File Structure](#15-file-structure)
16. [Troubleshooting & Exit Codes](#16-troubleshooting--exit-codes)

---

## 1. What Is VORTEX-OS?

VORTEX-OS is the **Native Autonomous Studio Command Center** for the MiniMax ecosystem. It is not a chatbot, and it is not a single AI model — it is a **Hierarchical Autonomous Orchestration Engine** that treats AI agency like a high-stakes, closed-loop corporate operation.

You give VORTEX-OS a single natural-language **master objective** — for example:

> *"Build a WebSim visual novel module for When Ocean Meets Sky — Mara on her porch in 1994, with a native audio loop, a prose scene, and an interactive HTML page. No smartphones. The radio inside plays only a weather report."*

VORTEX-OS then **decomposes** that objective across specialist domains, **dispatches** parallel worker agents in isolated sandboxes, **enforces** your creative rules via the Continuity Engine, **self-heals** when workers drift, **routes** audio to the native `minimax-music` engine, **generates** video via Hailuo, **processes** sound with local ffmpeg, **verifies** code in a sandbox, and **halts** for your approval before any high-stakes action.

**Everything happens in-house. No external API keys. No fragile websites. No dependency on third-party services.**

---

## 2. Why Native MiniMax?

| Problem | VORTEX-OS Solution |
|---|---|
| Fragile external APIs go down | 100% native MiniMax engines — minimax-music, Hailuo, minimax-image |
| External music sites change their terms | Local ffmpeg + Python audio scripts in sandbox — fully self-contained |
| Hallucinated canon violations | Continuity Engine catches & rewrites violations automatically |
| LLM drift on long projects | 4-tier chain of command isolates context per tier |
| High-stakes actions firing silently | Deep-Sleep HITL halts & pages you for approval |
| Repeated planning cost | Golden Path templates replay successful workflows for free |
| No audit trail | Every decision logged to `memory/audit.jsonl` |
| External video services are expensive | Native Hailuo pipeline generates video entirely in-house |
| Code runs differently in prod vs. dev | Sandbox Verifier checks every artifact before it leaves the swarm |

---

## 3. The 4-Tier Chain of Command

```
T0 — GENERAL MANAGER (The Apex)
       │  • Receives your master objective
       │  • Performs deep decomposition
       │  • Creates the master plan
       ▼
T1 — STORE SUPERVISOR (Domain Strategy)
       │  • Maintains the "Golden Path" project files
       │  • Routes to the right specialist swarm
       │  • Manages resource allocation
       ▼
T2 — SHIFT SUPERVISOR (Tactical QA)
       │  • Spawns workers in isolated memory sandboxes
       │  • Enforces the Continuity Engine (universe canon)
       │  • Runs sandbox verification on generated code
       │  • Holds the HITL checkpoint for high-stakes actions
       ▼
T3 — THE CREW (Specialized Workers)
       ├─── writer.docs          (prose, dialogue, narrative)
       ├─── media.native         (minimax-music audio, Hailuo video)
       ├─── coder.typescript     (HTML/CSS/JS, WebSim UIs, VibeOS)
       ├─── coder.python         (ffmpeg scripts, data pipelines, DSP)
       ├─── researcher.web       (web research, citations)
       ├─── analyst.strategic    (strategy, planning, risk)
       └─── designer.brand       (visual identity, continuity)
```

---

## 4. The 6 Architecture Pillars

1. **Hierarchical Decomposition** — 4-tier chain of command isolates context per tier, eliminates context rot.
2. **Continuity Engine** — Catches canonical violations (wrong character details, anachronisms, lore breaks) and forces rewrites.
3. **Self-Healing Prompt Optimizer** — DSPy-inspired: rewrites failing prompts to permanently eliminate failure modes.
4. **Native Engine Routing** — minimax-music for audio, Hailuo for video, minimax-image for art, local ffmpeg for DSP — no external services.
5. **Sandbox Verifier** — Ephemeral isolation checks generated code before it leaves the swarm.
6. **Human-in-the-Loop Gate (Deep-Sleep)** — High-stakes actions halt, suspend state to disk, page you for explicit approval.

---

## 5. Quick Start

```bash
cd skills/VORTEX-OS
chmod +x skill.sh verify.sh
bash verify.sh                  # Expected: ALL VERIFICATION CHECKS PASSED

./skill.sh --agents-discover    # List available agents
./skill.sh --agents-lint --all  # Lint all agents against the 8 invariants

# Submit your first master objective
mkdir -p my_project
cat > my_project/objective.md <<'EOF'
# Project: My First VibeOS Module

Build a WebSim scene: a coastal bedroom at sunset in 1994.

## Required deliverables
- 1 ambient audio loop (≈90 BPM, bedroom mood) via minimax-music
- 1 prose scene (200 words) for the protagonist
- 1 interactive HTML page with a fade-lights button

## Continuity rules
- No smartphones, no internet references
- The radio plays only a weather report
- Mara is 19, left-handed

## High-stakes
- package_websim: requires operator approval before final HTML write
EOF

./skill.sh --dispatch-master my_project/objective.md

# When HITL fires:
./skill.sh --hitl-status
./skill.sh --hitl-approve package_websim

# Inspect results:
ls deliverables/
./skill.sh --audit-trail
```

---

## 6. 200 Use Cases — What Can You Build?

All 200 use cases are organized by domain. Each one is a distinct project that maps to a single `--dispatch-master` call.

---

### Domain 1: Narrative & Worldbuilding (When Ocean Meets Sky / The Book of the Fading Age)

1. **Visual novel chapter** — Single scene with character dialogue, ambient audio loop, and interactive HTML page with 2 choice buttons
2. **Audio drama episode** — 3-voice script, 5 sound effects, a static HTML player with play/pause and a credits page
3. **Interactive CYOA book** — 10-paragraph story with 4 decision branches, a chapter selector, and a procedurally generated theme song
4. **Poetry collection** — 12 themed poems, an ambient soundscape, and a typographic HTML anthology site
5. **Screenplay scene** — A single 3-page screenplay, a mood-board image manifest, and a shot-list JSON for the director
6. **Short story submission** — 5,000-word literary story, a 200-word author bio, a cover letter, and a 280-character social post
7. **Comic book script** — 22-page script with panel descriptions, a dialog pass, a sound-design cue sheet, and a one-page pitch document
8. **Children's picture book** — 32 pages of text, a 12-second lullaby, a print-ready PDF layout spec, and a parental reading guide
9. **Choose-your-own-adventure gamebook** — 40 nodes, 12 endings, an item tracker JSON, and a printable character sheet
10. **Song album concept** — 10-song tracklist with lyrics, 10 album-cover art prompts, a 2,000-word liner notes essay, and a press release
11. **Show bible generation** — Auto-compile a 10-episode anime show bible, enforcing strict character arcs for Kai and Sora
12. **Lore consistency enforcement** — Audit a 100-chapter manuscript to ensure a character doesn't use an item they haven't found yet
13. **Branching dialogue trees** — Write complex, multi-path visual novel scripts and export them as structured JSON
14. **Constructed languages (Conlangs)** — Invent a sci-fi language with specific grammatical rules and a 500-word dictionary
15. **Non-Euclidean spatial mapping** — Generate scene descriptions for horror environments that deliberately violate the laws of physics
16. **NPC backstory factories** — Generate 50 unique tabletop RPG characters, complete with secrets and motivations
17. **Anachronism checking** — Scan a 1980s retro-futuristic script to ensure no modern technology is accidentally mentioned
18. **In-universe legal docs** — Draft highly detailed, fictional Non-Disclosure Agreements for cyberpunk mega-corporations
19. **Astrological lore tables** — Generate dynamic weather and celestial event calendars for a fictional universe
20. **Slang translation** — Rewrite standard dialogue into highly stylized, genre-specific slang (e.g., post-neuro street talk)

---

### Domain 2: Code Architecture & Internal App Brainstorming (VibeOS / 12matt3r Labs)

21. **Procedural logic blueprints** — Brainstorm the state-machine logic for a retro brawler combat loop to be coded inside MiniMax
22. **Aesthetic UI coding** — Generate modular HTML/CSS for randomized CRT glitch and VHS degradation effects triggered by user clicks
23. **Trippy cam math** — Write the WebGL shader logic for experimental visual effects that can be rendered in a local sandbox
24. **RPG stat balancing** — Auto-balance weapon stats across 500 items by running a simulated Python combat loop 10,000 times internally
25. **Infinite level algorithms** — Code procedural generation algorithms in Python for 2D mazes or dungeon crawlers
26. **Virtual OS logic** — Architecture the file-system simulation logic for a browser-based desktop environment (VibeOS)
27. **Hitbox prototyping** — Generate custom collision detection scripts in JavaScript for a 2.5D gaming environment
28. **AI pathfinding** — Write AI navigation logic for enemy NPCs with dynamic obstacle avoidance
29. **Save-state systems** — Compile a local-storage save management system script for narrative games
30. **Algorithmic UI layouts** — Brainstorm and code the flexbox grids for a retro-futuristic Memories Gallery
31. **Idle clicker game** — Full HTML/JS game loop, sound effects, an icon set spec, and a 200-word marketing blurb
32. **Puzzle game** — 20-level puzzle design doc, a working HTML prototype, ambient music per world, and a tutorial script
33. **Text adventure** — 30-room parser game, a map JSON, an ambient soundscape, and a player-facing walkthrough
34. **Mobile game spec** — 1-pager, 20-page GDD, a 5-level prototype, monetization notes, and an App Store description
35. **Tabletop RPG zine** — 24-page zine with 4 adventures, 12 NPCs, 18 magic items, a printable character sheet, and a backer update
36. **Visual novel engine** — A reusable WebSim dialogue system with branching, save/load, audio cues, and a sample 2-route scene
37. **Racing game prototype** — A playable 2D top-down racer with 3 tracks, engine sounds, a leaderboard, and a tuning menu
38. **Tower defense game** — 10-wave design, a working HTML prototype, 6 tower types, an enemy sprite spec, and a level editor spec
39. **Rhythm game** — A 5-track rhythm game with note charts (JSON), audio sync, scoring, and a difficulty curve analysis
40. **Roguelike prototype** — A 3-floor dungeon, a procedural generation algorithm spec, 12 enemy types, a permadeath save system

---

### Domain 3: Sound Design & Foley (Local ffmpeg / Sample Processing)

41. **Batch sample chopping** — Write and execute an ffmpeg bash script that slices a 10-minute field recording into 2-second percussive hits
42. **VHS audio degradation** — Generate the ffmpeg filters (lowpass, wow, flutter, distortion) to make clean dialogue sound like a damaged cassette
43. **Foley synthesis prompts** — Direct the internal text-to-speech engine to generate breathy, ethereal sighs and pitch-shift them down 12 semitones
44. **Dynamic stems splitting** — Write a Python script to isolate specific frequency bands of a snare sample for layered processing
45. **Reverb impulse responses** — Code the math to generate a synthetic impulse response WAV file mimicking a massive concrete silo
46. **Automated crossfading** — Use ffmpeg to seamlessly loop an atmospheric drone by overlapping the tail and head with a 50ms fade
47. **DSP visualizer math** — Generate Digital Signal Processing math for an audio visualizer reacting to bass frequencies
48. **Cassette tape emulation** — Write an ffmpeg script that injects procedural white noise and 60Hz hum under a voice track
49. **Pitch-ramp scripting** — Create a script that gradually drops the pitch and speed of an audio sample over 15 seconds to simulate a dying battery
50. **Granular synthesis logic** — Write a Python function that chops a sample into 10ms grains and reorganizes them randomly
51. **Podcast pilot script** — 30-minute episode script, an intro/outro jingle prompt, a show notes draft, and a 5-episode series bible
52. **Manifesto / philosophical tract** — 5,000-word manifesto, a typographic HTML presentation, an audio reading, and a discussion guide
53. **Graphic novel adaptation** — Convert a prose chapter into a 12-panel graphic-novel script with sound-cue annotations and an HTML reader
54. **D&D campaign arc** — 8-session campaign with NPCs, locations, plot beats, a soundtrack cue list, and a player handout packet
55. **Personal portfolio site** — Hero, 3 project showcases, an about page, an ambient background track, and a contact form
56. **Landing page for a SaaS** — Above-the-fold copy, 3 feature sections, testimonials, pricing, an explainer-video audio script
57. **Interactive resume** — A WebSim resume with skill cards, a timeline, downloadable PDF, and a voice-narration track
58. **Restaurant website** — Menu page, 3 photo gallery sections, reservation form, ambient dinner music, and a chef's bio
59. **Wedding website** — Story, schedule, RSVP, registry, a photo gallery, a "save the date" audio clip, and a guest book
60. **Nonprofit impact site** — Mission, 3 program pages, donation flow, impact metrics dashboard, a 60-second video script

---

### Domain 4: Native Video & Visual Rendering (Hailuo / MiniMax Vision)

61. **Batch image prompting** — Generate 50 highly specific, interconnected prompts for MiniMax's native image generator depicting "dark surrealism" architecture
62. **Video storyboarding** — Break down a narrative script into second-by-second camera angles and lighting directions for Hailuo video generation
63. **Logo system math** — Design the mathematical grid proportions for a retro-futuristic logo system (12matt3r Labs)
64. **Character reference sheets** — Auto-generate exact HEX color codes, outfit specs, and lighting conditions for character concept art
65. **Glitch video scripts** — Code a Python script to apply batch datamoshing effects to a folder of locally rendered MP4 files
66. **Cinematography directing** — Generate the exact text-to-video bracket commands (e.g., [pan left, slow zoom]) required to get the perfect camera move in Hailuo
67. **Brutalist asset generation** — Prompt the native image engine to generate harsh, Y2K-era UI elements (buttons, sliders, static)
68. **Automated watermarking** — Build a Python tool to automatically watermark and catalog generated artwork into a SQLite database
69. **Fictional terminal UI** — Design interface mockup assets for a retro hacking terminal screen
70. **Astrological color palettes** — Create a procedural color palette generator derived strictly from June 12th natal charts
71. **E-commerce single-product page** — Hero, 5 product shots, 3 testimonials, a buy button, and a product-demo video script
72. **Travel itinerary site** — 7-day trip plan, 12 attraction pages, an interactive map JSON, ambient sounds per city, and a packing list
73. **Educational web app** — 5-lesson interactive course with quizzes, progress tracking, audio narration, and a certificate generator
74. **News-style article** — Long-form investigative piece, 3 data visualizations, an audio reading, a pull-quote image spec, and a tweet thread
75. **Interactive fiction site** — A web-based CYOA reader with save states, an audio narrator, achievement tracking, and a stats page
76. **Conference site** — Speaker bios, schedule grid, sponsor tiers, ticket flow, a promo video script, and a swag spec
77. **Open-source project page** — README, 5 documentation pages, a logo spec, a quickstart tutorial, a contribution guide, and a demo
78. **Dashboard for a SaaS** — 4 KPI cards, 2 charts, a recent-activity table, a settings page, and a 60-second onboarding tour
79. **Job board** — Job posting form, listing page, application flow, an email-alert system spec, and an admin moderation panel
80. **Forum / community site** — Topic list, thread view, posting form, moderation tools, an audio welcome message, and a code-of-conduct page

---

### Domain 5: Software Automation & Pipeline Building

81. **Monolith to microservices** — Migrate a massive local Python script into cleanly separated, sandboxed modules
82. **API mocking** — Auto-generate a fully documented, locally hosted mock REST API using Python Flask for testing UI ideas
83. **Vector DB setup** — Build a local SQLite/Vector database architecture for a personal knowledge graph of your lore
84. **Algorithmic unit testing** — Write and execute a suite of unit tests for complex data-sorting functions
85. **Legacy code refactoring** — Automatically lint, format, and modernize a messy repository of old scripts
86. **Schema validation** — Write a script that checks all generated agent JSON manifests to ensure they follow VORTEX-OS rules
87. **Vulnerability scanning** — Audit a local codebase for logical vulnerabilities and automatically propose patches
88. **Auto-documentation** — Generate a custom Markdown site derived purely from source code comments
89. **Log file parsing** — Write a bash script that parses the audit.jsonl file to find the longest-running agent tasks
90. **File archiving** — Build a local Bash CLI tool to automate the zipping and timestamping of the deliverables folder
91. **Card game design** — A 60-card deck, rulebook, playtest feedback template, a quick-start guide, and an HTML rulebook site
92. **Board game adaptation** — Convert a video game to a physical board game: 30 components, a rulebook, a playtest report, and a box design
93. **Battle royale prototype** — A 50-player shrinking-zone game spec, a 5-minute match demo, audio cues, and a kill-feed UI
94. **SEO blog post** — 2,500-word pillar article, 5 supporting blog posts, 50 target keywords, meta descriptions, and an internal-link map
95. **Product launch campaign** — Launch announcement, 3 teaser posts, a press release, a launch-day live script, and a post-launch recap
96. **Whitepaper** — 12-page whitepaper with research, 3 data visualizations, a 1-page executive summary, an audio overview, and a CTA page
97. **Case study** — Customer story with a hero quote, before/after metrics, a 1-page PDF version, a 60-second video script, and a tweet
98. **Automated invoicing** — Write a Python script that auto-generates Markdown invoices and billing statements
99. **Brand style guides** — Create a definitive identity rulebook with strict typography, spacing, and brand-voice constraints
100. **SEO copywriting** — Write articles analyzing the intersection of AI generation and vaporwave aesthetics

---

### Domain 6: Research, Data & Text Processing

101. **Literature reviews** — Summarize 50 local text documents into a single comparative knowledge matrix
102. **Predictive statistics** — Build a predictive Python model for tracking daily habits vs. productivity
103. **Sentiment extraction** — Analyze your own chat logs to extract project momentum and behavioral patterns over a 6-month period
104. **Lore indexing** — Parse unstructured character notes into a clean, relational SQLite database
105. **Astrology dashboarding** — Build a local data dashboard script that cross-references astrological transits with your daily schedule
106. **Legalese translation** — Translate complex TOS agreements from software platforms into simple bullet points
107. **Competitive feature analysis** — Conduct a feature-by-feature breakdown of different UI layouts for a project
108. **Historical timelines** — Generate a highly detailed text timeline of a specific fictional era in your universe
109. **Manual condensation** — Synthesize a 50-page technical manual into a 2-page quick-start guide
110. **Regex generation** — Write complex Regular Expressions to instantly format and clean up messy, imported text files
111. **Online course** — 8-module course with lesson scripts, quizzes, audio intros, an instructor bio, a completion certificate
112. **Study guide for a textbook** — 200-question study guide, 50 flashcards, a 1-page cheat sheet, an audio summary, and a practice exam
113. **Language learning app** — 50 vocabulary cards, 20 grammar lessons, pronunciation audio, a speaking-practice spec, and a progress tracker
114. **Children's educational site** — 5 subjects, 25 activities, parental controls spec, audio narration, and a printable workbook
115. **Tutoring session plan** — A 60-minute lesson plan, 5 practice problems, an assessment rubric, a homework assignment, and parent notes
116. **Coding bootcamp curriculum** — 12-week curriculum with daily projects, code review rubrics, capstone spec, and alumni testimonials
117. **History lecture series** — 10-lecture syllabus with reading lists, discussion questions, slide-deck specs, an audio lecture script
118. **Math problem set generator** — 100 problems across 10 topics, step-by-step solutions, a difficulty-rating rubric, a printable answer key
119. **Science lab manual** — 15 experiments with safety notes, materials lists, expected results, a teacher edition, and a video demo script
120. **Curriculum for K-12** — A semester-long curriculum with daily lesson plans, assessments, parent communications, and a pacing guide

---

### Domain 7: Personal & Lifestyle (Third-Shift Optimization)

121. **Inverted meal prep logic** — Generate a strict nutritional algorithm optimizing for late-night energy crashes and morning sleep
122. **Routine synchronization** — Code a timeline generator that balances a night-shift job with daytime co-parenting schedules
123. **Boundary communication** — Auto-draft respectful, neutral boundary-setting messages for co-parenting conflict resolution
124. **Numerology calculators** — Build a personalized Python script that calculates life-path numbers instantly from input dates
125. **Gamified chore scripts** — Code a local Python script for a point-based chore and reward system for a child
126. **Offline task queues** — Create a terminal-based To-Do list that automatically prioritizes tasks based on your current energy level
127. **Finance tracking scripts** — Design a customized Python script to parse Stripe payout CSVs for Rosebud AI income
128. **Physical inventory DB** — Generate a local SQLite cataloging system for a real-world collection of VHS tapes and CRT monitors
129. **Circadian workouts** — Draft a workout routine mathematically optimized for circadian rhythm disruption
130. **Cosmic journaling generator** — Build a daily journaling prompt generator focused on philosophical alignment and soulmate connections
131. **Email campaign** — 7-email drip sequence with subject lines, body copy, CTA variations, an A/B test plan, and a send-time matrix
132. **Social media content pack** — 30-day content calendar, 60 captions, 20 hashtag sets, 10 reel scripts, and a brand voice guide
133. **Press release** — A 400-word press release, a quote bank, a media kit, a 280-character social announcement, and a journalist Q&A
134. **Sales pitch deck** — 12-slide pitch, a 1-page leave-behind, a 60-second elevator pitch audio, a competitive battlecard, and a demo script
135. **Grant proposal** — 20-page proposal with needs statement, methodology, budget, evaluation plan, organizational capacity, and a cover letter
136. **Strategic plan** — 3-year strategic plan with SWOT, OKRs, a roadmap timeline, a board presentation, and a stakeholder comms plan
137. **M&A due-diligence pack** — A 50-page due-diligence checklist, a financial model template, a legal review template, and a synergy analysis
138. **Crisis communication plan** — A 30-page crisis plan with 10 scenario playbooks, a media holding statement, a stakeholder matrix, and a response timeline
139. **Employee handbook** — 80-page handbook with policies, benefits, code of conduct, an onboarding checklist, a manager's guide, and a digital version
140. **Board meeting package** — 30-slide deck, a 10-page pre-read, financial summary, committee reports, a vote-tracker, and a minutes template

---

### Domain 8: Business, Branding & Pitching

141. **Investor pitch deck** — 15-slide deck, an executive summary, a 5-year financial projection model, a one-pager, and a Q&A prep doc
142. **Business plan** — 40-page plan with market analysis, financials, a go-to-market strategy, a competitor analysis, and a team page
143. **Quarterly report** — 30-page report with KPI dashboard, narrative analysis, a chairman's letter, a press release, and an earnings call script
144. **Collective pitch decks** — Write a complete, persuasive Markdown pitch deck for the 12matt3r collective
145. **Content calendars** — Generate a 30-day posting schedule for a new DriftWave STATIC release
146. **Hybrid resumes** — Draft specialized job applications that bridge physical labor (carpentry/landscaping) and high-tech AI orchestration skills
147. **Onboarding funnels** — Write the copy for an automated email sequence welcoming users to an experience design project
148. **Gig proposals** — Generate a comprehensive project proposal for a landscaping client integrating smart-home tech
149. **Virtual press releases** — Draft immersive press releases for digital domain events (like voidcity.live)
150. **Tagline brainstorming** — Generate 100 unique, punchy taglines for an experimental media agency
151. **Fitness app** — 12-week program, 50 exercise videos (script), a meal plan, a progress tracker, a notification system, and a coach bio
152. **Meal planning service** — 4-week meal plan, 60 recipes, a grocery list generator, a dietary substitution guide, an audio cooking tip per week
153. **Meditation app** — 30-session meditation library with scripts, 30 ambient audio cues, a streak tracker, a sleep-stories collection, and a settings UI
154. **Therapy practice site** — About page, 8 service descriptions, intake forms, an FAQ, a blog, a podcast page, and a HIPAA compliance checklist
155. **Wellness challenge** — 30-day challenge with daily tasks, an email check-in per day, a progress tracker, a community wall, and a final celebration
156. **Sleep story collection** — 15 bedtime stories for adults, ambient audio for each, a sleep timer UI, a favorites list, and a recommended-list page
157. **Sustainability report** — 40-page ESG report with metrics, narrative, third-party verification, an executive summary, and a printable one-pager
158. **Art exhibition catalog** — 30-artist catalog with bios, artist statements, an essay, a gallery layout plan, a wall-text template, and a press kit
159. **Photo book** — 80-page photo book with photo selection, captions, a forward, a layout plan, a cover design spec, and a print-ready PDF spec
160. **Magazine issue** — 60-page magazine with 8 feature articles, 4 interviews, a cover design, ad placements, a masthead, and a digital edition

---

### Domain 9: Meta-Orchestration (VORTEX-OS Self-Management)

161. **Agent synthesis** — Have the Store Supervisor design, code, and register a new specialized agent.devops.json manifest
162. **Adversarial red-teaming** — Run a "Red Team" swarm specifically ordered to try and break your own continuity rules to test system strength
163. **Auto-manuals** — Generate a complete VORTEX-OS user manual and troubleshooting guide based entirely on reading its own codebase
164. **Token auditing** — Write a Python script to mathematically analyze the memory/audit.jsonl and find inefficiencies in agent prompt structures
165. **CLI dashboarding** — Have a swarm build a graphical UI wrapper in Python (Tkinter/PyQt) to monitor VORTEX-OS execution in real-time
166. **Template translation** — Automatically translate old workflow templates into V4-compatible hierarchical JSON plans
167. **Bash optimization** — Have the system read its own lib/ files and optimize the bash scripts for faster execution times
168. **Tournament mode** — Spawn three coding agents, give them the same problem, test which code runs fastest, and log the winner
169. **Stress testing datasets** — Generate a synthetic dataset of 1,000 mock prompts to stress-test the HITL checkpoint system
170. **Automated backups** — Create a self-executing cron protocol for backing up the orchestrator's state and SQLite memory files locally
171. **Literature review** — Summarize 30-paper annotated bibliography, a 5,000-word synthesis, a methodology section, a research-question matrix, and a citation map
172. **Market research report** — 50-page report with TAM/SAM/SOM, 20 customer interviews, 5 competitor profiles, a SWOT, and an executive summary
173. **User research synthesis** — 25-interview synthesis with persona cards, a journey map, 10 insight statements, a recommendation deck, and a roadmap
174. **Patent application** — 25-page patent with claims, abstract, drawings spec, prior-art analysis, an inventor's declaration, and a filing checklist
175. **Lab protocol** — 30-page protocol with materials, safety notes, step-by-step procedure, expected results, a troubleshooting guide, and a data sheet
176. **Scientific paper** — 12-page paper with abstract, intro, methods, results, discussion, 5 figures, a supplementary info section, and a cover letter
177. **Competitive intelligence brief** — 20-competitor matrix, a feature comparison, a pricing analysis, market positioning map, and strategic recommendations
178. **Brand identity kit** — Logo concepts, color palette, typography, brand voice guide, business card mockup, and a 30-second brand anthem
179. **Style guide** — 60-page style guide with design tokens, component library, accessibility checklist, code snippets, and a Figma spec
180. **Mood board** — A 20-image mood board, a 500-word creative brief, a color palette, a typography suggestion, and a 1-page design direction

---

### Domain 10: Deep Creative & Experimental

181. **Pixel-to-data scripts** — Write a local Python script that converts the pixel data of an image into raw hex codes to inspire visual UI palettes
182. **Procedural poetry** — Write a script that generates surrealist poetry whose meter and tone change based on a local random number seed
183. **Terminal text adventures** — Build a "choose your own adventure" text game entirely run and played inside the MiniMax sandbox command line
184. **Simulated economies** — Create a virtual economy logic script where agents trade a finite number of "tokens" for the right to optimize prompts
185. **Philosophical simulators** — Auto-generate endless, deeply nuanced philosophical debates between simulated historical figures, saving the transcripts to disk
186. **ARG generation** — Design an Alternate Reality Game complete with cryptic text files, base64 encrypted clues, and lore directories
187. **Algorithmic video editing math** — Write a Python script that calculates the exact timestamp cuts required to edit random video clips based on audio transients
188. **Smart carpentry blueprints** — Generate text-based architectural blueprints for real-world carpentry projects that include hidden compartments for tech hardware
189. **Automated tarot** — Build a digital Tarot card reading Python script that contextualizes the pull against your stored astrological data
190. **The Ultimate Archive** — Compile all generated knowledge, rules, code, and lore into a searchable, offline local JSON database
191. **Wedding planner toolkit** — 12-month checklist, vendor comparison sheets, budget tracker, a guest list manager, a seating chart tool, and a vendor email pack
192. **Mystery dinner party kit** — 8-character script, an evidence pack, audio cues, a host guide, an invitation, and a costume suggestion sheet
193. **Astronomy outreach program** — 12-month stargazing calendar, a beginner's guide to the night sky, a 10-page moon atlas, an audio tour, and a planet fact sheet
194. **Recipe + cocktail pairing app** — 30 dinner recipes, 30 cocktail pairings, a music playlist per menu, a shopping list, a printable menu card, and a hosting timeline
195. **Interactive museum exhibit** — 12-station walkthrough, an audio tour script, a 3D-modeled artifact manifest, an interactive kiosk spec, a teacher's guide, and a souvenir booklet
196. **Lookbook** — 24-page fashion lookbook with 20 outfits, styling notes, behind-the-scenes captions, a soundtrack, and a runway spec
197. **Real estate listing site** — Property details, 12 photo gallery, a virtual tour script, neighborhood data, and a contact form
198. **Stage play** — 90-minute 3-act play, a prop list, a lighting cue sheet, a character relationship graph, and a director's note
199. **Podcast episode pack** — 12-episode podcast season with scripts, intro/outro music, show notes, guest booking templates, and a sponsor pitch deck
200. **Multi-language localization kit** — Translate a single interactive narrative into 10 languages, with audio localization scripts, a glossary, and a cultural adaptation guide

---

## 7. The Complete Command Reference

### Discovery & Inspection
| Command | Purpose |
|---|---|
| `./skill.sh --agents-discover` | List all available agents |
| `./skill.sh --agents-inspect <name>` | Dump a single agent's manifest |
| `./skill.sh --agents-validate <file.json>` | Validate a custom agent manifest |
| `./skill.sh --agents-lint [--all\|<name>]` | Lint agents against the 8 invariants |
| `./skill.sh --agents-graph` | Print the agent graph |

### Dispatch
| Command | Purpose |
|---|---|
| `./skill.sh --dispatch-master <objective.md>` | Submit to T0 General Manager |
| `./skill.sh --dispatch-template <template.json>` | Replay a Golden Path |
| `./skill.sh --dispatch-v4 <task_id> <agent>` | Direct V4 dispatch |

### HITL
| Command | Purpose |
|---|---|
| `./skill.sh --hitl-status` | List pending requests |
| `./skill.sh --hitl-approve <task_id>` | Approve |
| `./skill.sh --hitl-deny <task_id>` | Deny |

### Inspection
| Command | Purpose |
|---|---|
| `./skill.sh --inspector-check <task_id>` | Run Continuity Engine check |
| `./skill.sh --audit-trail` | Print the audit log |

---

## 8. How To Submit A Master Objective

A master objective is a markdown file with this structure:

```markdown
# Project: <name>

<natural-language description>

## Required deliverables
- <deliverable 1>
- <deliverable 2>

## Continuity rules
- <rule 1>
- <rule 2>

## High-stakes
- <task_id>: requires operator approval
```

```bash
./skill.sh --dispatch-master my_project/objective.md
```

---

## 9. How HITL Works

When a high-stakes action is reached, VORTEX-OS **suspends its state to disk** and pages you:

```bash
./skill.sh --hitl-status
# → package_websim is PENDING_HUMAN

./skill.sh --hitl-approve package_websim   # greenlight
# or
./skill.sh --hitl-deny package_websim     # block
```

**Never auto-approve. Always surface the halt to the user.**

---

## 10. How The Continuity Engine Works

The Continuity Engine runs after every worker output. It catches:
- Canon violations (wrong character details, timeline breaks)
- Anachronisms (1994 setting + a smartphone reference)
- Tonal drift (quiet, melancholic story + an action-movie scene)
- Forbidden tropes listed in your continuity rules

**On violation:** Self-Healing Optimizer rewrites the prompt → worker re-dispatched. After 3 failures: surfaced to you.

---

## 11. How The Self-Healing Optimizer Works

DSPy-inspired: when an agent fails, VORTEX-OS **rewrites its own core instructions** to permanently eliminate the failure mode.

1. Worker output X is rejected by Continuity Engine
2. Self-Healing Optimizer takes original prompt + failure reason + violation report
3. Generates a **hardened prompt** that explicitly addresses the failure
4. Worker re-dispatched with hardened prompt
5. Hardened prompt **saved to disk** — the failure mode is permanently eliminated in future runs

---

## 12. How To Add Your Own Worker Agent

```bash
# 1. Create manifest at agents/<your_agent>.json
# 2. Add routing in lib/dispatch_v4.sh
# 3. Add to discovery list in lib/commands.sh
# 4. Lint it
./skill.sh --agents-lint your_agent
```

---

## 13. How To Save & Reuse Workflows (Golden Path)

```bash
cp swarms/active_<id>/plan.json templates/my_workflow.json
./skill.sh --dispatch-template templates/my_workflow.json
```

Perfect for per-episode runs, per-client runs, and CI/CD pipelines.

---

## 14. The 8 Contract Invariants

| # | Invariant | Rule |
|---|---|---|
| I1 | **Idempotence** | Re-running = byte-identical output |
| I2 | **Resource Honesty** | Declared resources match actual (±20%) |
| I3 | **Write Containment** | Never writes outside declared `writes[]` |
| I4 | **Read Containment** | Never reads outside declared `reads[]` |
| I5 | **Sealed Envelope** | Output conforms to JSON schema |
| I6 | **Retry Honesty** | Never loops internally |
| I7 | **Secret Hygiene** | No secrets in logs |
| I8 | **Metric Truthfulness** | Metrics are actual, not estimated |

---

## 15. File Structure

```
skills/VORTEX-OS/
├── README.md                              ← ★ The knowledge base (first thing you see)
├── INSTRUCTIONS.md                        ← ★ LLM operator knowledge base
├── SKILL.md                               ← LLM instruction brain
├── _meta.json                             ← Platform registration
├── skill.sh                               ← CLI entry point
├── verify.sh                              ← Post-upload verification
│
├── lib/                                   ← Engine (6 modules)
│   ├── swarm.sh                           ← T1/T2 coordination
│   ├── hitl.sh                            ← Deep-Sleep HITL gate
│   ├── inspector.sh                        ← Continuity Engine + invariants
│   ├── prompt_optimizer.sh               ← Self-Healing Optimizer
│   ├── dispatch_v4.sh                     ← T3 worker dispatch
│   └── commands.sh                        ← Command functions
│
├── agents/                                ← 3 supervisor manifests
│   ├── supervisor.store.json
│   ├── supervisor.shift.json
│   └── inspector.governance.json
│
└── state/  swarms/  tasks/  memory/  deliverables/  ← Runtime (created on first run)
```

---

## 16. Troubleshooting & Exit Codes

| Exit Code | Meaning | Action |
|---|---|---|
| `0` | Success | Read `deliverables/` |
| `2` | Bad input / missing file | Check the file path |
| `42` | Continuity violation unresolved after 3 rewrites | Inspect `state/inspector_interventions.log` |
| `100` | Invariant lint failure | Run `./skill.sh --agents-lint --all` |
| `203` | HITL pending | Run `./skill.sh --hitl-status` |
| `127` | Command not found | Install `jq`, `sqlite3`, or `ffmpeg` |

**Common fix:** `apt install jq sqlite3 ffmpeg`

---

## License

MIT

## Author

MiniMax Agent
