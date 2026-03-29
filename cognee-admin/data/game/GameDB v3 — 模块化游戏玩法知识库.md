# GameDB v3 — 模块化游戏玩法知识库

> 基于 Jesse Schell《游戏设计艺术》模块化设计理念构建的结构化游戏设计知识库。
> 将游戏拆解为可组合的"玩法原子"，通过"配方"组合成完整品类，并叠加"跨切面系统"。

## 架构概览

```
gamedb-v3/
├── atoms/           # L1: 玩法原子（228 个）
│   ├── md/          #     人类可读 Markdown 文档
│   ├── json/        #     机器可读 JSON 数据
│   └── rs/          #     Rust 公式代码（可执行）
├── recipes/         # L2: 品类配方（50 个 Archetype）
│   ├── md/          #     配方 Markdown 文档
│   └── json/        #     配方 JSON 数据
├── systems/         # L3: 跨切面系统（15 个）
│   ├── md/          #     系统 Markdown 文档
│   └── json/        #     系统 JSON 数据
└── meta/            # 元数据与索引
```

## 统计数据

| 层级 | 类型 | 数量 | 格式 | 文件总数 |
| :--- | :--- | ---: | :--- | ---: |
| L1 | 玩法原子 | 228 | .md + .json + .rs | 684 |
| L2 | 品类配方 | 50 | .md + .json | 100 |
| L3 | 跨切面系统 | 15 | .md + .json | 30 |
| **合计** | | **293 个知识单元** | | **814 个文件** |

## L1: 玩法原子（228 个）

### A. Core Movement & Traversal（21 个）

| atom_id | 英文名 | 中文名 |
| :--- | :--- | :--- |
| atom_variable_jump | Variable Jump Height | 可变跳跃高度 |
| atom_coyote_time | Coyote Time | 土狼时间 |
| atom_jump_buffer | Jump Buffering | 跳跃预输入 |
| atom_halved_gravity_peak | Halved-Gravity Jump Peak | 跳跃顶点半重力 |
| atom_corner_correction | Corner Correction | 拐角修正 |
| atom_wall_slide_jump | Wall Slide & Jump | 滑墙与墙跳 |
| atom_double_jump | Double Jump | 二段跳 |
| atom_air_dash | Air Dash | 空中冲刺 |
| atom_ground_pound | Ground Pound | 下砸 |
| atom_crouch_slide | Crouch Slide | 蹲滑 |
| atom_grapple_hook | Grapple Hook / Tether | 钩锁/绳索 |
| atom_glide_float | Glide / Float | 滑翔/漂浮 |
| atom_swim_dive | Swim / Dive | 游泳/潜水 |
| atom_climb_system | Climb System | 攀爬系统 |
| atom_momentum_storage | Momentum Storage | 动量存储 |
| atom_teleport_blink | Teleport / Blink | 瞬移/闪现 |
| atom_dodge_roll | Dodge Roll / Dash | 翻滚/闪避 |
| atom_sprint_stamina | Sprint with Stamina Cost | 冲刺消耗体力 |
| atom_vehicle_mount | Vehicle / Mount Riding | 载具/坐骑骑乘 |
| atom_drift_boost | Drift Boost | 漂移加速 |
| atom_slipstream | Slipstream / Drafting | 尾流跟车 |

### B. Combat & Action（38 个）

| atom_id | 英文名 | 中文名 |
| :--- | :--- | :--- |
| atom_melee_combo | Melee Combo Chain | 近战连招链 |
| atom_iframes | Invincibility Frames | 无敌帧 |
| atom_stamina_combat | Stamina-Gated Combat | 体力门控战斗 |
| atom_poise_hyperarmor | Poise / Hyper Armor | 韧性/霸体 |
| atom_parry_riposte | Parry / Riposte | 弹反/处决 |
| atom_witch_time | Witch Time / Slow-Mo Dodge | 魔女时间/慢动作闪避 |
| atom_style_meter | Style Meter / Rank | 风格评分 |
| atom_weapon_switch | Real-Time Weapon Switch | 实时武器切换 |
| atom_juggle_launch | Juggle / Launch | 浮空连击 |
| atom_cancel_system | Animation Cancel | 动画取消 |
| atom_lock_on | Lock-On Targeting | 锁定目标 |
| atom_hitscan | Hitscan Shooting | 即时命中射击 |
| atom_projectile | Projectile Shooting | 弹道射击 |
| atom_ads | Aim Down Sights | 举枪瞄准 |
| atom_recoil_pattern | Recoil Pattern | 后坐力模式 |
| atom_reload_mechanic | Reload Mechanic | 换弹机制 |
| atom_headshot_multi | Headshot Multiplier | 爆头倍率 |
| atom_weapon_bloom | Weapon Bloom / Spread | 弹道散布 |
| atom_ability_cooldown | Ability Cooldown | 技能冷却 |
| atom_ultimate_charge | Ultimate Charge | 大招充能 |
| atom_elemental_reaction | Elemental Reaction | 元素反应 |
| atom_summon_companion | Summon / Companion | 召唤/伙伴 |
| atom_qte | Quick Time Event | 快速反应事件 |
| atom_stealth_takedown | Stealth Takedown | 潜行处决 |
| atom_vision_cone | Vision Cone Detection | 视锥检测 |
| atom_alert_states | Alert State Machine | 警戒状态机 |
| atom_disguise | Disguise System | 伪装系统 |
| atom_distraction | Distraction / Lure | 分散注意力/引诱 |
| atom_light_shadow | Light & Shadow Stealth | 光影潜行 |
| atom_body_hiding | Body Hiding | 藏尸 |
| atom_rps_counter | Rock-Paper-Scissors Counter | 剪刀石头布克制 |
| atom_action_points | Action Points | 行动点数 |
| atom_cover_system | Cover System | 掩体系统 |
| atom_hit_probability | Hit Probability | 命中概率 |
| atom_overwatch_reaction | Overwatch / Reaction Fire | 监视/反应射击 |
| atom_flanking_bonus | Flanking Bonus | 侧翼加成 |
| atom_weapon_triangle | Weapon Triangle | 武器三角 |
| atom_aggro_threat | Aggro / Threat | 仇恨值 |

### C. Puzzle & Logic（16 个）

| atom_id | 英文名 | 中文名 |
| :--- | :--- | :--- |
| atom_spatial_portal | Spatial Portal | 空间传送门 |
| atom_physics_puzzle | Physics-Based Puzzle | 物理谜题 |
| atom_pattern_match | Pattern Matching | 图案匹配 |
| atom_swap_match3 | Swap Match-3 | 交换三消 |
| atom_cascade_chain | Cascade / Chain Reaction | 连锁反应 |
| atom_special_piece | Special Piece Generation | 特殊块生成 |
| atom_block_rotation | Block Rotation / Placement | 方块旋转/放置 |
| atom_logic_deduction | Logic Deduction | 逻辑推理 |
| atom_evidence_contradiction | Evidence Contradiction | 证据矛盾 |
| atom_sokoban_push | Sokoban Push | 推箱子 |
| atom_pipe_connect | Pipe / Circuit Connection | 管道/电路连接 |
| atom_word_puzzle | Word Puzzle | 文字谜题 |
| atom_hidden_object | Hidden Object Search | 隐藏物品搜索 |
| atom_escape_room | Escape Room Puzzle | 密室逃脱谜题 |
| atom_memory_match | Memory Match | 记忆配对 |
| atom_fishing_minigame | Fishing Minigame | 钓鱼小游戏 |

### D. Resource & Economy（33 个）

| atom_id | 英文名 | 中文名 |
| :--- | :--- | :--- |
| atom_currency_dual | Dual Currency System | 双货币系统 |
| atom_energy_stamina_gate | Energy / Stamina Gate | 体力门控 |
| atom_gacha_banner | Gacha Banner / Pull | 抽卡卡池 |
| atom_pity_system | Pity System | 保底系统 |
| atom_battle_pass | Battle Pass | 战斗通行证 |
| atom_auction_house | Auction House / Market | 拍卖行/市场 |
| atom_crafting_recipe | Crafting Recipe | 制造配方 |
| atom_crafting_grid | Crafting Grid | 合成网格 |
| atom_resource_gathering | Resource Gathering | 资源采集 |
| atom_sink_faucet | Sink / Faucet Economy | 水槽/水龙头经济 |
| atom_trade_npc | NPC Trade / Shop | NPC交易/商店 |
| atom_insurance | Insurance System | 保险系统 |
| atom_item_durability | Item Durability | 物品耐久度 |
| atom_inventory_grid | Grid Inventory (Tetris) | 网格背包(俄罗斯方块式) |
| atom_inventory_weight | Weight-Based Inventory | 负重背包 |
| atom_loot_rarity | Loot Rarity Tier | 装备稀有度 |
| atom_random_affix | Random Affix / Roll | 随机词缀 |
| atom_socket_gem | Socket / Gem System | 镶嵌/宝石系统 |
| atom_equipment_slot | Equipment Slot | 装备栏位 |
| atom_merge_upgrade | Merge / Combine Upgrade | 合并/合成升级 |
| atom_ad_reward | Ad Reward | 广告奖励 |
| atom_monthly_card | Monthly Card / Subscription | 月卡/订阅 |
| atom_card_draft | Card Draft | 卡牌选择 |
| atom_deck_thinning | Deck Thinning | 卡组精简 |
| atom_energy_per_turn | Energy Per Turn | 每回合能量 |
| atom_draw_discard | Draw / Discard Pile | 抽牌堆/弃牌堆 |
| atom_card_upgrade | Card Upgrade | 卡牌升级 |
| atom_relic_artifact | Relic / Artifact | 遗物/神器 |
| atom_exhaust_banish | Exhaust / Banish Card | 消耗/放逐卡牌 |
| atom_star_merge | Star-Up / Merge Units | 升星/合成单位 |
| atom_interest_economy | Interest Economy | 利息经济 |
| atom_shared_draft | Shared Draft / Carousel | 旋转选秀 |
| atom_limited_resources | Limited Resources | 有限资源 |

### E. Progression & Meta（23 个）

| atom_id | 英文名 | 中文名 |
| :--- | :--- | :--- |
| atom_xp_level | XP / Level Up | 经验值/升级 |
| atom_skill_tree | Skill Tree | 技能树 |
| atom_tech_tree | Tech Tree | 科技树 |
| atom_class_job | Class / Job System | 职业系统 |
| atom_passive_web | Passive Skill Web | 被动技能网 |
| atom_constellation_dupe | Constellation / Dupe System | 命座/重复系统 |
| atom_prestige_reset | Prestige / Ascension Reset | 声望/飞升重置 |
| atom_meta_progression | Meta-Progression | 元进度 |
| atom_achievement_system | Achievement / Trophy | 成就/奖杯 |
| atom_star_rating | Star Rating | 星级评分 |
| atom_streak_system | Daily Streak | 每日连续打卡 |
| atom_season_rank | Seasonal Rank / Ladder | 赛季排名/天梯 |
| atom_mastery_rank | Mastery / Proficiency | 熟练度/精通 |
| atom_respec | Respec / Skill Reset | 洗点/重置 |
| atom_new_game_plus | New Game Plus | 二周目 |
| atom_daily_weekly_reset | Daily / Weekly Reset | 日常/周常重置 |
| atom_offline_progress | Offline Progress | 离线进度 |
| atom_collection_log | Collection Log / Codex | 收藏图鉴 |
| atom_sanity_system | Sanity System | 理智系统 |
| atom_score_chase | Score Chase / Leaderboard | 分数追逐/排行榜 |
| atom_skin_cosmetic | Skin / Cosmetic Unlock | 皮肤/外观解锁 |
| atom_tool_upgrade | Tool Upgrade | 工具升级 |
| atom_synergy_trait | Synergy / Trait System | 羁绊/特质系统 |

### F. Narrative & Social（25 个）

| atom_id | 英文名 | 中文名 |
| :--- | :--- | :--- |
| atom_branching_dialogue | Branching Dialogue | 分支对话 |
| atom_dialogue_wheel | Dialogue Wheel | 对话轮盘 |
| atom_morality_system | Morality System | 道德系统 |
| atom_affinity_meter | Affinity / Relationship Meter | 好感度计量 |
| atom_timed_choice | Timed Choice | 限时选择 |
| atom_fmv_choice | FMV Interactive Choice | 全动态影像选择 |
| atom_butterfly_effect | Butterfly Effect | 蝴蝶效应 |
| atom_multiple_endings | Multiple Endings | 多结局 |
| atom_skill_check | Skill Check / Dice Roll | 技能检定 |
| atom_inner_voice | Inner Voice / Thought Cabinet | 内心声音/思维柜 |
| atom_environmental_storytelling | Environmental Storytelling | 环境叙事 |
| atom_journal_lore | Journal / Lore Entry | 日志/传说条目 |
| atom_photo_mode | Photo Mode | 拍照模式 |
| atom_role_assignment | Role Assignment (Social) | 角色分配(社交) |
| atom_discussion_vote | Discussion & Voting | 讨论与投票 |
| atom_sabotage | Sabotage / Betrayal | 破坏/背叛 |
| atom_emote_gesture | Emote / Gesture | 表情/手势 |
| atom_guild_clan | Guild / Clan System | 公会/氏族系统 |
| atom_gift_giving | Gift Giving | 送礼 |
| atom_coop_revive | Co-op Revive | 合作复活 |
| atom_pvp_arena | PvP Arena / Duel | PvP竞技场/决斗 |
| atom_spectator_mode | Spectator Mode | 观战模式 |
| atom_holy_trinity | Holy Trinity (Tank/Heal/DPS) | 神圣三角 |
| atom_dungeon_finder | Dungeon Finder / Matchmaking | 副本匹配 |
| atom_diplomacy | Diplomacy System | 外交系统 |

### G. Structure & Level Design（48 个）

| atom_id | 英文名 | 中文名 |
| :--- | :--- | :--- |
| atom_procedural_gen | Procedural Generation | 程序化生成 |
| atom_permadeath | Permadeath | 永久死亡 |
| atom_run_based | Run-Based Session | 单局制结构 |
| atom_node_map | Node Map / Path Selection | 节点地图/路径选择 |
| atom_wave_system | Wave System | 波次系统 |
| atom_shrinking_zone | Shrinking Zone / Circle | 毒圈收缩 |
| atom_extraction_zone | Extraction Zone | 撤离区域 |
| atom_checkpoint | Checkpoint / Bonfire | 检查点/篝火 |
| atom_save_point | Save Point / Save Room | 存档点/安全房间 |
| atom_ability_gating | Ability Gating | 能力门控 |
| atom_interconnected_map | Interconnected Map | 互联地图 |
| atom_fast_travel | Fast Travel | 快速旅行 |
| atom_day_night_cycle | Day/Night Cycle | 日夜循环 |
| atom_seasonal_calendar | Seasonal Calendar | 季节日历 |
| atom_weather_system | Weather System | 天气系统 |
| atom_biome_system | Biome System | 生态群落 |
| atom_tower_placement | Tower Placement | 塔防放置 |
| atom_tower_upgrade_path | Tower Upgrade Path | 塔升级路线 |
| atom_target_priority | Targeting Priority | 目标优先级 |
| atom_zoning_system | Zoning System | 分区系统 |
| atom_rci_demand | RCI Demand Balance | RCI需求平衡 |
| atom_traffic_sim | Traffic Simulation | 交通模拟 |
| atom_belt_conveyor | Belt / Conveyor System | 传送带系统 |
| atom_colonist_ai | Colonist AI / Needs | 殖民者AI/需求 |
| atom_storyteller_ai | Storyteller / AI Director | 叙事者/AI导演 |
| atom_board_movement | Board Movement (Dice) | 棋盘移动(掷骰) |
| atom_minigame_rotation | Minigame Rotation | 小游戏轮换 |
| atom_elimination_round | Elimination Round | 淘汰赛 |
| atom_endless_mode | Endless / Infinite Mode | 无尽模式 |
| atom_instant_restart | Instant Restart | 即时重开 |
| atom_chapter_replay | Chapter Replay | 章节重玩 |
| atom_hunger_thirst | Hunger / Thirst | 饥饿/口渴 |
| atom_temperature_exposure | Temperature / Exposure | 温度/暴露 |
| atom_shelter_building | Shelter Building | 庇护所建造 |
| atom_block_placement | Block Placement / Destruction | 方块放置/破坏 |
| atom_fog_of_war | Fog of War | 战争迷雾 |
| atom_unit_production | Unit Production Queue | 单位生产队列 |
| atom_supply_cap | Supply / Population Cap | 人口上限 |
| atom_board_positioning | Board Positioning | 棋盘站位 |
| atom_raid_mechanic | Raid Mechanic | 团本机制 |
| atom_chase_sequence | Chase Sequence | 追逐序列 |
| atom_victory_conditions | Multiple Victory Conditions | 多种胜利条件 |
| atom_culture_spread | Culture / Religion Spread | 文化/宗教传播 |
| atom_io_growth | IO Growth (Eat to Grow) | IO成长(大鱼吃小鱼) |
| atom_crop_growth | Crop Growth Stages | 作物生长阶段 |
| atom_festival_event | Festival / Seasonal Event | 节日/季节活动 |
| atom_work_priority | Work Priority Queue | 工作优先级 |
| atom_mood_mental_break | Mood / Mental Break | 心情/精神崩溃 |

### H. Input & Presentation（24 个）

| atom_id | 英文名 | 中文名 |
| :--- | :--- | :--- |
| atom_one_tap | One-Tap Control | 单键控制 |
| atom_swipe_gesture | Swipe Gesture | 滑动手势 |
| atom_tilt_gyro | Tilt / Gyroscope | 倾斜/陀螺仪 |
| atom_touch_drag | Touch & Drag | 触摸拖拽 |
| atom_note_highway | Note Highway | 音符轨道 |
| atom_timing_window | Timing Window Judgment | 判定窗口 |
| atom_hold_note | Hold Note | 长按音符 |
| atom_combo_counter | Combo Counter | 连击计数 |
| atom_score_multiplier | Score Multiplier | 分数乘数 |
| atom_ghost_replay | Ghost Replay | 幽灵回放 |
| atom_rubber_banding_ai | Rubber Banding AI | 橡皮筋AI |
| atom_outfit_scoring | Outfit Scoring | 服装评分 |
| atom_tag_matching | Tag / Attribute Matching | 标签/属性匹配 |
| atom_room_decoration | Room / Space Decoration | 房间/空间装饰 |
| atom_furniture_catalog | Furniture Catalog | 家具目录 |
| atom_feeding_care | Feeding / Care Cycle | 喂养/照顾循环 |
| atom_pet_evolution | Pet Growth / Evolution | 宠物成长/进化 |
| atom_notification_reminder | Notification / Reminder | 通知/提醒 |
| atom_spaced_repetition | Spaced Repetition | 间隔重复 |
| atom_adaptive_difficulty | Adaptive Difficulty | 自适应难度 |
| atom_control_group | Control Group | 编队控制 |
| atom_flashlight | Flashlight / Light Source | 手电筒/光源 |
| atom_audio_cue | Audio Cue / Spatial Sound | 音频提示/空间音效 |
| atom_swerve_dodge | Swerve / Dodge Obstacles | 闪避障碍 |

## L2: 品类配方（50 个 Archetype）

| archetype_id | 英文名 | 中文名 |
| :--- | :--- | :--- |
| souls-like | Souls-like | 魂类游戏 |
| zelda-like | Zelda-like | 塞尔达类 |
| diablo-like | Diablo-like | ARPG刷宝类 |
| metroidvania | Metroidvania | 银河城类 |
| hades-like | Hades-like | Roguelite动作类 |
| deckbuilder-roguelike | Deckbuilder Roguelike | 卡组构建Rogue类 |
| dress-up | Dress-Up | 换装类 |
| cooking-game | Cooking Game | 烹饪类 |
| character-action | Character Action | 角色动作类 |
| precision-platformer | Precision Platformer | 精确平台跳跃 |
| classic-vn | Classic Visual Novel | 经典视觉小说 |
| tower-defense | Tower Defense | 塔防类 |
| auto-battler | Auto-Battler | 自动战棋类 |
| match-3 | Match-3 | 三消类 |
| logic-puzzle | Logic Puzzle | 逻辑解谜类 |
| physics-puzzle | Physics Puzzle | 物理解谜类 |
| party-game | Party Game | 派对游戏 |
| social-deduction | Social Deduction | 社交推理类 |
| hack-slash | Hack & Slash | 破破烂烂类 |
| bullet-hell | Bullet Hell | 弹幕类 |
| twin-stick | Twin-Stick Shooter | 双摇杆射击 |
| wave-survival | Wave Survival | 波次生存类 |
| tycoon | Tycoon | 大亨经营类 |
| colony-sim | Colony Sim | 殖民地模拟 |
| farming-life | Farming Life Sim | 农场生活模拟 |
| city-builder | City Builder | 城市建造类 |
| survival-crafting | Survival Crafting | 生存制造类 |
| base-building | Base Building | 基地建设类 |
| turn-based-strategy | Turn-Based Strategy | 回合制策略 |
| rts | Real-Time Strategy | 即时战略 |
| clicker-idle | Clicker / Idle | 点击/放置类 |
| survival-horror | Survival Horror | 生存恐怖类 |
| walking-sim | Walking Simulator | 步行模拟类 |
| dating-sim | Dating Sim | 恋爱模拟类 |
| point-click | Point & Click Adventure | 点击冒险类 |
| 2d-fighter | 2D Fighter | 2D格斗类 |
| arena-brawler | Arena Brawler | 竞技场乱斗 |
| arcade-racing | Arcade Racing | 街机竞速类 |
| rhythm-action | Rhythm Action | 音乐动作类 |
| stealth-action | Stealth Action | 潜行动作类 |
| sports-sim | Sports Simulation | 体育模拟类 |
| sandbox-creative | Sandbox Creative | 沙盒创造类 |
| virtual-pet | Virtual Pet | 虚拟宠物类 |
| decoration | Decoration | 装饰类 |
| hidden-object | Hidden Object | 隐藏物品类 |
| escape-room | Escape Room | 密室逃脱类 |
| 4x-strategy | 4X Strategy | 4X策略类 |
| endless-runner | Endless Runner | 无尽跑酷类 |
| open-world-rpg | Open World RPG | 开放世界RPG |
| jrpg-turnbased | JRPG Turn-Based | JRPG回合制 |

## L3: 跨切面系统（15 个）

| system_id | 英文名 | 中文名 |
| :--- | :--- | :--- |
| sys_f2p_energy | F2P Energy Economy | F2P体力经济 |
| sys_gacha_monetization | Gacha Monetization | Gacha变现系统 |
| sys_battle_pass | Battle Pass System | 战斗通行证系统 |
| sys_season_live_ops | Seasonal Live Operations | 赛季运营系统 |
| sys_matchmaking | Matchmaking & ELO | 匹配与评分系统 |
| sys_netcode | Netcode Architecture | 网络架构 |
| sys_dynamic_difficulty | Dynamic Difficulty Adjustment | 动态难度调整 |
| sys_achievement_trophy | Achievement & Trophy System | 成就与奖杯系统 |
| sys_social_guild | Social & Guild System | 社交与公会系统 |
| sys_ugc_modding | UGC & Modding System | UGC与Mod系统 |
| sys_tutorial_onboarding | Tutorial & Onboarding | 新手引导系统 |
| sys_save_system | Save & Persistence System | 存档与持久化系统 |
| sys_accessibility | Accessibility System | 无障碍系统 |
| sys_procedural_content | Procedural Content Generation | 程序化内容生成 |
| sys_economy_sink_faucet | Virtual Economy (Sink/Faucet) | 虚拟经济(水槽/水龙头) |

## AI Agent 使用指南

### 组合规则（五步法）

1. **美学驱动**：确定目标游戏的核心美学体验（如"挑战感"、"叙事沉浸"）
2. **原子选择**：从 228 个原子中选择匹配的原子，参考现有配方
3. **接口匹配**：检查原子间的 `compatible_atoms` 和 `incompatible_atoms`
4. **系统注入**：根据商业模式叠加跨切面系统
5. **冲突检测**：验证原子组合不存在设计矛盾

### 快速组合示例

```
《原神》= atom_dodge_roll + atom_elemental_reaction + atom_ability_cooldown
       + atom_gacha_banner + atom_pity_system + atom_energy_stamina_gate
       + atom_daily_weekly_reset + atom_constellation_dupe
       + sys_gacha_monetization + sys_battle_pass + sys_season_live_ops
```

## 版本信息

- **版本**: v3.0
- **构建日期**: 2026-03-27
- **文件总数**: 814
- **知识单元**: 293（228 原子 + 50 配方 + 15 系统）
