## 逐音比较与局部采用（Cycle 202）

生成多个方案后打开“逐音比较方案”。对照方案只供比较，切换它不会替换当前修改；表格按音轨和音符身份列出当前编辑与对照指法的差异，可按小节查看，逐项选择或选择当前小节全部差异，定位到实际指法输入位置。

“采用选中差异”只改变勾选位置。其他手工修改与未采用位置保留，双手窗口的另一只手也保留。编辑、切换当前方案或勾选采用范围变化后，需要“重新比较当前编辑结果”，不能继续采用过期快照。

单手采用后点“按修改重新推荐”：把选择的手指固定后重新求解，保留未勾选位置，完成实际保存审阅才可接受。双手采用后重新“核对双手保存结果”；混合方案可能发生持音或手指冲突，必须按实际检查处理，不能直接把两个可行整段方案的局部拼接当成可行。

## 多音轨共同编辑（Cycle 158）

在“共同编辑的同手音轨”勾选其他声部，便可一起推荐和保存。表格逐行显示所属音轨，修改手指后按“按修改重新推荐”固定重算；不同轨的第一个音符可以分别固定。未选声部继续作为参考，已有指法保持固定。范围内主轨休止不会漏掉其他所选声部。

“保存后的实际指法”逐轨显示接受、保留、未标记、新标记和改已有数量，可以取消某轨选择或接受该轨全部。取消选择不删除其已有指法；未标记位置不填入没勾选的推荐。局部选择仍需确认，一笔接受同时保存所有勾选轨，撤销上次指法修改恢复整笔。保存也记忆实际接受声部的手型。

只共同编辑同一只手的声部，另一只手请另行推荐。每次最多 32 条目标轨、4096 个目标音符，并保持有界持音上下文；复杂冲突需调整声部/固定指法/选段，不能把有限搜索视为教师审定。

## 同手分轨联动（Cycle 157）

同一只手分散在多条音轨时，推荐自动参考附近其他同手音轨的同时音和仍在按住的音符。其他轨已标记手指保持固定，未标记位置参与方案预览；只保存所选音轨的勾选建议。不同手和未指定手的音轨不自动混入。

手位预览包含联动音符，声音示范也包含选段里的同手声部，各自使用原轨的通道/音量设置。跨轨同拍同键共用手指；如果固定标记冲突，先修改已有标记或分手再推荐。其他轨指法/手别改变后需重新生成方案。

保存结果按各音轨实际标记检查。其他同手声部有未标记音符时，即使所选轨全部接受，也需确认保留这些空位；不会代为保存参考指法。需要为另一轨确认指法时，切到该音轨另行推荐、审阅和保存。共同编辑/批量保存及未标记位置的完整衔接确认仍待继续实现。

## 保存结果审阅（Cycle 156）

勾选接受的音符后，“保存后的实际指法”会合并这些建议和未勾选位置的个人标记，显示接受、保留和未标记数量。未标记位置不会填入没接受的推荐。局部接受前勾选确认；每次修改选择都重新审阅并重置确认。

当前音轨、所选手出现同一手指同时按不同键，或持音/同时按键的指序冲突时，本次推荐不能保存。手位可以切换“完整推荐方案”和“保存后的实际指法”，未标记显示“·”。点选冲突自动显示实际组合，直接看到保留与新手指；可补选完整方案，或“固定已有指法后重新选择建议”再生成。既有标记不会因审阅或失败保存而改变，成功的一笔仍可撤销。服务器在保存前重新核对源数据和实际标记组合。

审阅还列出实际保存组合的跨度和快速换指。检查仅涉及已标记位置和当前音轨/演奏手，未标记音符的完整衔接、多音轨同手和身体动作仍需进一步判断。直接人工编辑仍可自行标注。

## 按速度推荐与难点复核（Cycle 155）

“推荐使用速度”选择原速的 25% 至 200%，按实际发音间隔计算快速穿/跨指、同指换键和移手代价，并按该速度识别同音快速重复。选段与原音符身份不变；这个设置不改变练习速度。示范声音仍通过独立的“示范速度”控制。

每个方案列出需要复核的位置：快速穿/跨指、快速移手、同指换键，以及当前和弦或先前持音造成的跨度。已有固定指法也检查。点击某条位置会停止声音示范、显示对应手位并定位到指法表格；可调整手型、速度、分手，或固定手指再重新计算。

快速穿/跨指的 180 毫秒、移手/同指换键的 250 毫秒和手型半音跨度属于公开显示的启发式阈值，不是个人能力测量或教师审定。暂未建模身体动作、持音换指、连断奏与专家样例校准；未发现上述难点不等于推荐完美。只保存实际接受的音符，局部接受仍需复核与未接受位置的衔接。

## 整段指法示范（Cycle 143）

生成建议并选好方案后，点击“示范整个建议选段”。示范速度可选择 25%、50%、75% 或原速，按原来的节奏发音、同时演奏和弦并保留起点持音，键盘上的方案指法实时跟随；只示范指定手/音轨的选段。声音沿当前音源及音轨音量输出，静音设置仍有效。

示范不改变练习位置、不计入成绩、不录制也不保存指法；接受并保存仍是独立操作。先停止练习/录音/计划/阶梯并松开琴键。停止按钮、切换方案、点选表格音符、重新计算、按下实际琴键或关闭窗口都会停止。修改手指后先重新推荐，不能用旧方案示范未计算的修改。

示范帮助比较手位衔接，专家校准、身体动作和持音换指模型仍待完善，不能把按键数字当作完整身体教学。

## 备选与确认（Cycle 126）

打开“指法建议”，选择音轨、手型和小节范围，生成至多三个不同路径。比较路径代价、穿/跨指、移手、宽手型和已有指法改动。搜索有上限，结果不代表教师审定或全局最佳。

“固定选段内已有指法”决定是否保留选段中的个人标记；选段外标记始终保留。修改某个建议手指后，点击“按修改重新推荐”把它作为锚点，重新计算周围位置，再接受保存。相矛盾的持音、和弦或锚点会报告小节；请调整固定手指、分手或范围。

键盘手位显示当前位置和仍按住的音符，前后按钮逐位置查看，表格音符按钮直接跳转。保存前会检查建议生成以来的音符、分手和已有指法变化。接受一笔可撤销，手型选择在接受后保存。局部接受只修改勾选音符；未保存位置的实际衔接需要另外检查。

乐谱点选后的“推荐此音指法”使用同一套搜索，可以选择此音的备选手指；只保存此音时不等于保存整个和弦方案。完整比较使用“和弦 / 选段建议”。

# Explainable fingering suggestions

Neothesia can propose one finger or one complete same-onset chord shape while
the manual finger editor is active. Suggestions are previews, never automatic
edits:

1. Open **Edit fingers** for an imported MIDI.
2. Select a note with `Left` / `Right`.
3. Press `G` to preview a suggestion.
4. Read the finger, confidence and reason.
5. Press `Enter` to accept, choose `1`–`5` yourself, or move on. On a chord,
   Enter accepts the complete visible shape in one save.

Only acceptance writes the content-bound sidecar.

The selected unassigned note carries a cyan dot directly on the waterfall.
After `G`, that dot becomes the cyan preview finger. On a chord, every proposed
digit appears cyan together. Accepting writes the whole group atomically;
unselected saved numbers return to white while the current selection stays
cyan. Reviewed hand-turn landings remain gold. Cyan, white and gold therefore
mean selection/preview, saved guidance and technical turn respectively.

## Why a cost model

Automatic fingering is a combinatorial problem with multiple valid answers.
The prototype follows the established approach of representing successive
finger choices as a path and using dynamic programming to find the lowest-cost
path. The model is deliberately small enough that every selected transition
can be classified and explained.

The research basis:

- Al Kasimi, Nichols and Raphael describe dynamic programming over a
  user-adjustable fingering cost function:
  <https://ismir2007.ismir.net/posters/ISMIR2007_p355_kasimi_poster.pdf>
- Nakamura and colleagues emphasize both the usefulness of constraint/cost
  methods and the real individual variation between pianists:
  <https://doi.org/10.1016/j.ins.2019.12.068>
- Baylor Piano Basics teaches the keyboard-shape rule used by the static key
  cost: fingers 2–3 cover two-black-key groups, 2–3–4 cover three-black-key
  groups, and thumbs normally take white keys:
  <https://openbooks.library.baylor.edu/pianobasics/chapter/d-flat-and-g-flat-major-scales/>

These references justify an explainable prototype, not a claim that its output
is an editorial or teacher-approved fingering.

## Current cost terms

The dynamic program evaluates all five fingers at every modeled note and
penalizes:

- changing finger on a slow or isolated repeated pitch;
- moving against the natural finger order without a thumb/finger crossing;
- using the thumb, and to a lesser extent finger 5, on a black key;
- stretching farther than a conservative finger-pair span;
- crossing across a large interval;
- keeping one finger while changing pitch.

It rewards:

- stable fingers on slow/isolated repeated notes;
- finger order that follows the melodic direction for the selected hand;
- ordinary in-position movement;
- thumb-under and finger-over turns when continuation needs them;
- previously saved manual hints, which act as fixed contextual anchors.

The currently selected note is intentionally unanchored so the learner can ask
for a genuine alternative.

## Rapid repeated notes

Repeated-note fingering is contextual, not one universal rule:

- Indiana University Press's open *Class Piano* text presents changing fingers
  on a repeated note as one available fingering device, while emphasizing
  individual hand fit and consistent practice once a choice is made:
  <https://publish.iupress.indiana.edu/read/class-piano/section/d57966c3-dc98-4825-b143-0f8c5f58173f>
- Youmee Kim's Ohio State performance study says alternating fingers is
  usually easier for fast repeated notes, but documents musical/acoustic
  exceptions involving melody emphasis, wide movement and narrow black keys:
  <https://etd.ohiolink.edu/acprod/odb_etd/ws/send_file/send?accession=osu1199061624&disposition=inline>

The prototype therefore alternates only when:

- at least three consecutive notes share one pitch; and
- each relevant onset gap is at most 250 ms.

The dynamic program starts from a balanced finger and gives adjacent changes
the lowest cost. Four rapid middle C attacks can therefore preview 3–2–1–2.
Two isolated attacks or a 500 ms repeated run keep the same finger. Manual
anchors remain hard constraints. The explanation is “alternates fingers so
rapid repeated notes can release cleanly,” at a deliberately moderate 76%
confidence because articulation, key color and musical accent are not yet
modeled.

## Chord shapes

Two- through five-note chords on one hand track receive a vertical shape
suggestion. The model:

- sorts the simultaneous notes by pitch without depending on MIDI event order;
- enumerates every unique, hand-ordered subset of fingers 1–5;
- rejects assignments that contradict a saved manual anchor;
- compares pitch spacing with the physical spacing of the candidate fingers;
- applies the selected hand-span profile and black-key costs;
- returns the lowest-cost legal shape to all notes in the chord.

For example, a close-position C–E–G triad produces right-hand 1–3–5 and
left-hand 5–3–1. This is a transparent geometric starting point, not an
editorial claim for every inversion, voicing or musical phrase.

Six-note clusters, duplicate pitches inside one track/onset and contradictory
anchors return no suggestion. A chord wider than the selected profile still
shows the obvious ordered outer-finger shape at low confidence, with the
explicit warning “do not force the reach.” The learner can roll, redistribute
or omit the chord instead.

The editor keeps the whole shape pending under one preview transaction. `Enter`
updates all exact-note hints in memory and performs one atomic sidecar
replacement, so a failed write cannot leave half a chord saved. Direct `1`–`5`
input remains a single-note override.

### Consecutive chords

Adjacent chord onsets are optimized as a short state sequence instead of
choosing every vertical shape independently. Each chord still contributes its
full ergonomic shape cost. A smaller transition cost then:

- prefers a common pitch to remain under the same finger;
- penalizes unnecessary movement of a finger used in both shapes;
- labels a retained common tone with the reason “keeps a common chord tone
  under the same finger.”

The transition term is intentionally a light tie-breaker. It must not turn an
ordinary open hand shape into a cramped 1–2–3 merely to preserve a finger. For
example, E–G–C and D–G–B are both allowed to use 1–3–5 so their common G remains
under finger 3; the isolated first chord's also-plausible 1–2–5 loses only
because the two vertical options are close.

This is chord-to-chord voice-leading assistance, not held-note substitution.
The model does read each MIDI note's key-release time. A prior chord tone whose
note-off occurs after the next onset keeps its assigned finger occupied:

- that finger cannot simultaneously play a different new pitch;
- new notes must remain on the anatomically correct side of every held
  note/finger pair for the selected hand;
- valid new assignments explain that they use fingers still free around held
  harmony;
- if no candidate can respect those constraints, the affected onset receives
  no suggestion instead of asking for an impossible hand shape.

The same occupancy continues into a single-note melody tail immediately after
the chord. Every new note must remain clear of the held fingers; after the
first note, ordinary melodic transition cost also resumes so an ascending A–B
does not repeat one merely “available” finger. The tail stops using the chord
constraint when the last held chord tone reaches note-off.

This still does not invent finger substitution. Moving from one finger to
another while one key remains depressed is a distinct action that ordinary
note-on/note-off MIDI does not encode by itself.

## Hand-span personalization

Open **Settings → Practice** and set **Right Hand Span** and **Left Hand Span**
independently. Each hand can use:

- **Compact · up to a 7th** for smaller hands or learners who should reposition
  instead of being encouraged into broad stretches;
- **Standard · up to an octave**, the backward-compatible default;
- **Large · up to a 9th** for pianists who can comfortably cover wider shapes.

Each setting is persistent and changes the comfortable distance assigned to
that hand's finger pairs. Notes beyond that distance cost progressively more,
so the lowest-cost path can choose a position shift or different finger
pattern. It does not prohibit a large interval: melodic leaps can still require
a shift, and the preview remains advice rather than an anatomical safety
assessment.

Settings written before per-hand profiles keep their shared value as the
fallback for both hands. A previous Compact choice therefore migrates as
Compact/Compact, not Standard/Standard. Changing one new row creates only that
hand's override. Settings from before hand-span support default to Standard.

## Reasons and confidence

Every preview reports one of:

- saved manual anchor;
- balanced phrase start;
- repeated note;
- rapid repeated-note alternation;
- in-position movement;
- thumb-under;
- finger-over;
- position shift after a leap.

Confidence is a communication tier tied to the transition type, not a
statistical probability. Manual anchors report 100%; repeated and in-position
choices are high; crossings are medium; phrase starts and large shifts are
lower because hand size and musical context matter more.

## Honest boundary

Version 1 works on tracks already classified as left or right hand. Melodic
runs use a sequential dynamic program; simultaneous two- through five-note
groups use an independent vertical hand-shape model. Tracks with ambiguous
hand ownership receive no suggestion.

Future work can add:

- finer anatomy calibration beyond three span categories;
- explicit held-note substitutions and repeated-note alternation;
- phrase/slur and articulation context;
- comparison against expert-annotated datasets;
- alternative suggestions instead of only the lowest-cost path.


## 双手共同审阅

从指法建议点击“双手一起审阅”，设置相同小节范围和推荐速度。各手使用自己的跨度条件，并联合全部已分配给该手的音轨生成建议。可以分别选择候选路径、逐音修改编号或取消接受，再点击“核对双手保存结果”。跨度选择用于本次推荐，初始值读取已有手型偏好。

核对结果分别显示两手接受、保留、未标记、上下文未标记及需要复核的位置。未分手音符和双手同时同音需人工确认；部分接受也需确认保留结果。两手不共享同一套手指编号限制，手内同时按键与持音指序分别检查。手交叉和身体动作的联合优化仍未实现。

修改后需重新核对，保存前也会重新检查音符、分手及实际资料。任意一手无效都不会只保存另一手；两手一笔保存，撤销恢复整笔。关闭未保存窗口会提示。保存时先停止演奏、录音和活动计划，并松开按键。


## 双手慢练段

核对双手保存结果后，可以“整理双手难点练习段”；没有算法难点时可“建立双手练习段”。重叠的小节合并，名称、小节、速度、重复次数和备注都能修改。可扩展到相邻小节练衔接，但需包含本次共同审阅的目标音。目标声部的未分手音符需先处理。

两手的目标声部共同评分，其余声部恢复伴奏或静音。段落保留预备拍、节拍器、延迟、手别及重复设置，可以从练习段落重开，也能加入计划或备份。相同编排重复保存会提示已存在；后来修改过的段落不会被旧编排覆盖。这个操作使用当前已有指法，仍须单独确认未接受的指法建议。

## 双手同步示范

核对双手保存结果并完成必要确认后，点击“示范双手审阅结果”。示范使用共同小节范围，左右手按同一节奏发音，包括范围起点仍按住的音。下面分别显示两只手正在演奏的音名和手指；休止时显示休止。速度独立选择 25%、50%、75% 或原速。

这次勾选及修改的指法参与显示，未勾选位置保留已有标记；尚未标记的音明确显示“未标记”。已有持音换指也会随时间更新；若候选指法改变了换指动作的起始条件，或所选速度不适用，需先保存指法并重新审阅动作。

示范不保存指法，不改变练习位置或成绩。停止按钮、改变范围/候选/指法、关闭或隐藏窗口、开始练习及实际按键都会停止；停止后可再次播放。音轨静音和音源设置沿用当前条件。

### 暂停、定位和键盘查看

双手示范下方按选段音域显示黑白键，蓝色表示左手，橙色表示右手，数字是当前手指，问号表示未标记。换指后的编号会更新。键盘用于查看实际按键位置。

点击“暂停示范”可保留当前音名和指法，“继续示范”从原处播放。在“定位小节”直接选择实际演奏的第几小节，也可以拖动示范位置；暂停时定位仍保持暂停，播放时定位继续播放。定位与续播会恢复该位置仍在按住的音。

示范结束后可以选较早小节再次续播，也可重新从头示范。暂停时同样可以停止；修改审阅结果或关闭窗口会结束这次示范。


## 两手交接与音域审阅

“双手一起审阅 → 核对双手保存结果”会同时检查两手按键区间。同键即使起音不同，只要一只手尚未离键，另一只手开始按同一键，也会列入交接审阅。前一手离键恰好与下一手起音相同时不列为重叠；踏板延音与实际持键分别处理。

左手某个音高于同时演奏的右手音时，列为音域交叠位置，供核对分手、手位与两手路径。这个检查来自音符区间，身体姿势和动作优化仍由人工判断。

可以按同键交接或音域交叠筛选，逐组查看实际小节、两手音轨/音符和按推荐速度计算的重叠时长。“查看两手指法位置”切换到对应指法页并定位；范围前已经按住的音需扩大小节范围后修改。截断时缩小范围继续审阅。

确认交接位置后，“整理双手难点练习段”会把它们按小节编排，默认速度最多 50%（已选更慢速度继续保留）和三遍，保存具体两手交接要求。可以编辑名称、范围、速度、次数和备注，再保存并从练习段落目录打开，或加入练习计划。这个操作保留现有指法，指法建议仍需单独确认保存。
