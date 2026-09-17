# Method inventory: 04-objects

> Audit status (04-objects): `[x]`=ported/equivalent, `[>]`=partial (see
> `.spec/base-audit/reports/04-objects.md`), `[ ]`=missing, `[~]`=not needed
> (UI/HUD, backend list management, Theme, data-dead).
> Summary: 161 defs — [x]=41, [>]=51, [ ]=12, [~]=57 (counts verified).

## object1.cc (75 defs)

- [x] `object1.cc:64` **apply_flags_set**(s16b a_idx, s16b set_idx, object_flag_set *f)
- [x] `object1.cc:177` **object_flavor**(object_colors_t const &object_colors, std::shared_ptr<object_kind> k_ptr)
- [x] `object1.cc:242` **object_easy_know**(std::shared_ptr<object_kind> k_ptr) — Certain items, if aware, are known instantly This function is used only by "flavor_init()" XXX XXX XXX Add "EASY_KNOW" flag to "k_info.txt" file — synced from report
- [x] `object1.cc:327` **flavor_init**() — Prepare the "variable" part of the "k_info" array. The "color"/"metal"/"type" of an item is its "flavor". For the most part, flavors are assigned randomly each game. Initialize descriptions for the "c — synced from report
- [~] `object1.cc:383` **reset_visuals**() — [~] 物品选择/展示窗口（show_*/get_item*/display_*）：port 由 modal.rs 选择模态与隐式过滤器（闭包）承担；accept.py item-select 类机械复核
- [~] `object1.cc:449` **object_flags_xtra**(object_type const *o_ptr, object_flag_set *f) — [~] object_flag_set/pval mask 位集内部：port 用字符串旗标 + EquipTotals/item_flags（item.rs）；accept.py flag-internals 类机械复核
- [x] `object1.cc:542` **object_flags**(object_type const *o_ptr) — Obtain the "flags" for an item — synced from report
- [~] `object1.cc:625` **object_flags_known**(object_type const *o_ptr) — [~] object_flag_set/pval mask 位集内部：port 用字符串旗标 + EquipTotals/item_flags（item.rs）；accept.py flag-internals 类机械复核
- [x] `object1.cc:683` **calc_object_need_exp**(object_type const *o_ptr) — Calculate amount of EXP needed for the given object to level, assuming it's a sentient object. — synced from report
- [~] `object1.cc:692` **compute_pval_mask**() — [~] object_flag_set/pval mask 位集内部：port 用字符串旗标 + EquipTotals/item_flags（item.rs）；accept.py flag-internals 类机械复核
- [x] `object1.cc:752` **object_desc_aux**(object_type const *o_ptr, int pref, int mode) — Creates a description of the item "o_ptr", and stores it in "out_val". One can choose the "verbosity" of the description, including whether or not the "number" of items should be described, and how mu — synced from report
- [x] `object1.cc:1715` **object_desc**(char *buf, object_type const *o_ptr, int pref, int mode) — synced from report
- [~] `object1.cc:1727` **object_desc_store**(char *buf, object_type *o_ptr, int pref, int mode) — [~] 对象信息屏文字（text_out）→ port 的 Item::label/observe 文本（modal.rs observe_text）；accept.py terminal-UI 类机械复核
- [x] `object1.cc:1752` **item_activation**(object_type *o_ptr) — Determine the "Activation" (if any) for an artifact Return a string, or NULL for "no activation" — synced from report
- [x] `object1.cc:1793` **grab_tval_desc**(int tval) — [x] bevy/src/item.rs grab_tval_desc → tval_desc（47 条表）；observe 屏使用；测试 tval_desc_covers_the_original_table
- [~] `object1.cc:1810` **check_first**(bool *first) — [~] 物品选择/展示窗口（show_*/get_item*/display_*）：port 由 modal.rs 选择模态与隐式过滤器（闭包）承担；accept.py item-select 类机械复核
- [~] `object1.cc:1824` **output_dam**(object_type *o_ptr, int mult, int mult2, const char *against, const char *against2, bool *first) — [~] 对象信息屏文字（text_out）→ port 的 Item::label/observe 文本（modal.rs observe_text）；accept.py terminal-UI 类机械复核
- [~] `object1.cc:1865` **display_weapon_damage**(object_type *o_ptr) — [~] 物品选择/展示窗口（show_*/get_item*/display_*）：port 由 modal.rs 选择模态与隐式过滤器（闭包）承担；accept.py item-select 类机械复核
- [~] `object1.cc:1915` **output_ammo_dam**(object_type *o_ptr, int mult, int mult2, const char *against, const char *against2, bool *first) — [~] 对象信息屏文字（text_out）→ port 的 Item::label/observe 文本（modal.rs observe_text）；accept.py terminal-UI 类机械复核
- [~] `object1.cc:1966` **display_ammo_damage**(object_type *o_ptr) — [~] 物品选择/展示窗口（show_*/get_item*/display_*）：port 由 modal.rs 选择模态与隐式过滤器（闭包）承担；accept.py item-select 类机械复核
- [~] `object1.cc:2021` **describe_device**(object_type *o_ptr) — Describe a magic stick powers — synced from report
- [x] `object1.cc:2066` **object_out_desc_where_found**(s16b level, s16b dungeon) — Helper for object_out_desc Print the level something was found on — synced from report
- [~] `object1.cc:2100` **object_out_desc**(object_type *o_ptr, FILE *fff, bool trim_down, bool wait_for_it) — Describe an item — synced from report
- [x] `object1.cc:3062` **index_to_label**(int i) — Convert an inventory index into a one character label Note that the label does NOT distinguish inven/equip. — synced from report
- [x] `object1.cc:3076` **label_to_inven**(int c) — [x] bevy/src/item.rs label_to_inven（A2I + 包内校验）；测试 labels_map_to_inventory_and_equipment
- [x] `object1.cc:3098` **label_to_equip**(int c) — [x] bevy/src/item.rs label_to_equip（包后接装备标签）；测试 labels_map_to_inventory_and_equipment
- [~] `object1.cc:3119` **get_slot**(int slot) — [~] 对象信息屏文字（text_out）→ port 的 Item::label/observe 文本（modal.rs observe_text）；accept.py terminal-UI 类机械复核
- [x] `object1.cc:3152` **wield_slot_ideal**(object_type const *o_ptr, bool ideal) — [x] bevy/src/modal.rs wield_slot_ideal（沿用 wield_slot_for 的槽规则，object1.cc:3152）
- [x] `object1.cc:3333` **wield_slot**(object_type const *o_ptr) — Determine which equipment slot (if any) an item likes for the player's current body and stuff — synced from report; `Modal::wield_apply` now uses `wield_slot_for` (modal.rs:13378) with `mimic::slot_usable_body` for the base slot, a same-type extra slot fallback and the "no body part" refusal.
- [~] `object1.cc:3341` **mention_use**(int i) — [~] 物品选择/展示窗口（show_*/get_item*/display_*）：port 由 modal.rs 选择模态与隐式过滤器（闭包）承担；accept.py item-select 类机械复核
- [~] `object1.cc:3444` **describe_use**(int i) — Return a string describing how a given item is being worn. Currently, only used for items in the equipment, not inventory. — synced from report
- [~] `object1.cc:3544` **item_tester_okay**(object_type const *o_ptr, object_filter_t const &filter) — [~] 物品选择/展示窗口（show_*/get_item*/display_*）：port 由 modal.rs 选择模态与隐式过滤器（闭包）承担；accept.py item-select 类机械复核
- [~] `object1.cc:3571` **show_equip_aux**(bool mirror, object_filter_t const &filter) — [~] 物品选择/展示窗口（show_*/get_item*/display_*）：port 由 modal.rs 选择模态与隐式过滤器（闭包）承担；accept.py item-select 类机械复核
- [~] `object1.cc:3572` **show_inven_aux**(bool mirror, object_filter_t const &filter) — [~] 物品选择/展示窗口（show_*/get_item*/display_*）：port 由 modal.rs 选择模态与隐式过滤器（闭包）承担；accept.py item-select 类机械复核
- [~] `object1.cc:3577` **display_inven**() — [~] 物品选择/展示窗口（show_*/get_item*/display_*）：port 由 modal.rs 选择模态与隐式过滤器（闭包）承担；accept.py item-select 类机械复核
- [~] `object1.cc:3587` **display_equip**() — [~] 物品选择/展示窗口（show_*/get_item*/display_*）：port 由 modal.rs 选择模态与隐式过滤器（闭包）承担；accept.py item-select 类机械复核
- [~] `object1.cc:3595` **get_item_letter_color**(object_type const *o_ptr) — [~] 物品选择/展示窗口（show_*/get_item*/display_*）：port 由 modal.rs 选择模态与隐式过滤器（闭包）承担；accept.py item-select 类机械复核
- [~] `object1.cc:3618` **show_inven_aux**(bool mirror, const object_filter_t &filter) — [~] 物品选择/展示窗口（show_*/get_item*/display_*）：port 由 modal.rs 选择模态与隐式过滤器（闭包）承担；accept.py item-select 类机械复核
- [~] `object1.cc:3781` **show_inven**(object_filter_t const &filter) — [~] 物品选择/展示窗口（show_*/get_item*/display_*）：port 由 modal.rs 选择模态与隐式过滤器（闭包）承担；accept.py item-select 类机械复核
- [~] `object1.cc:3786` **show_inven_full**() — [~] 物品选择/展示窗口（show_*/get_item*/display_*）：port 由 modal.rs 选择模态与隐式过滤器（闭包）承担；accept.py item-select 类机械复核
- [~] `object1.cc:3793` **show_equip**(object_filter_t const &filter) — [~] 物品选择/展示窗口（show_*/get_item*/display_*）：port 由 modal.rs 选择模态与隐式过滤器（闭包）承担；accept.py item-select 类机械复核
- [~] `object1.cc:3798` **show_equip_full**() — [~] 物品选择/展示窗口（show_*/get_item*/display_*）：port 由 modal.rs 选择模态与隐式过滤器（闭包）承担；accept.py item-select 类机械复核
- [~] `object1.cc:3808` **show_equip_aux**(bool mirror, object_filter_t const &filter) — [~] 物品选择/展示窗口（show_*/get_item*/display_*）：port 由 modal.rs 选择模态与隐式过滤器（闭包）承担；accept.py item-select 类机械复核
- [~] `object1.cc:4039` **toggle_inven_equip**() — [~] 物品选择/展示窗口（show_*/get_item*/display_*）：port 由 modal.rs 选择模态与隐式过滤器（闭包）承担；accept.py item-select 类机械复核
- [~] `object1.cc:4080` **verify**(const char *prompt, int item) — [~] 物品选择/展示窗口（show_*/get_item*/display_*）：port 由 modal.rs 选择模态与隐式过滤器（闭包）承担；accept.py item-select 类机械复核
- [~] `object1.cc:4107` **get_item_allow**(int item) — [~] 物品选择/展示窗口（show_*/get_item*/display_*）：port 由 modal.rs 选择模态与隐式过滤器（闭包）承担；accept.py item-select 类机械复核
- [~] `object1.cc:4144` **get_item_okay**(int i, object_filter_t const &filter) — [~] 物品选择/展示窗口（show_*/get_item*/display_*）：port 由 modal.rs 选择模态与隐式过滤器（闭包）承担；accept.py item-select 类机械复核
- [~] `object1.cc:4167` **get_tag**(int *cp, char tag) — [~] 物品选择/展示窗口（show_*/get_item*/display_*）：port 由 modal.rs 选择模态与隐式过滤器（闭包）承担；accept.py item-select 类机械复核
- [~] `object1.cc:4263` **show_floor**(int y, int x, object_filter_t const &filter) — [~] 物品选择/展示窗口（show_*/get_item*/display_*）：port 由 modal.rs 选择模态与隐式过滤器（闭包）承担；accept.py item-select 类机械复核
- [~] `object1.cc:4365` **get_item_floor**(int *cp, const char *pmt, const char *str, int mode, object_filter_t const &filter, select_by_name_t const &select_by_na...) — [~] 物品选择/展示窗口（show_*/get_item*/display_*）：port 由 modal.rs 选择模态与隐式过滤器（闭包）承担；accept.py item-select 类机械复核
- [~] `object1.cc:5080` **get_item**(int *cp, const char *pmt, const char *str, int mode, object_filter_t const &filter, select_by_name_t const &select_by_na...) — [~] 物品选择/展示窗口（show_*/get_item*/display_*）：port 由 modal.rs 选择模态与隐式过滤器（闭包）承担；accept.py item-select 类机械复核
- [x] `object1.cc:5089` **item_tester_hook_getable**(object_type const *o_ptr) — Hook to determine if an object is getable
- [x] `object1.cc:5102` **wear_ammo**(object_type *o_ptr) — Wear a single item from o_ptr
- [x] `object1.cc:5169` **pickup_ammo**() — Try to pickup arrows
- [x] `object1.cc:5209` **can_carry_heavy**(object_type const *o_ptr) — Check for encumberance if player were to pick up given item. — synced from report
- [x] `object1.cc:5233` **object_pickup**(int this_o_idx) — Do the actuall picking up — synced from report
- [x] `object1.cc:5301` **absorb_gold**(cave_type const *c_ptr) — synced from report
- [x] `object1.cc:5339` **sense_floor**(cave_type const *c_ptr) — synced from report
- [x] `object1.cc:5365` **py_pickup_floor**(int pickup) — synced from report
- [x] `object1.cc:5476` **gain_flag_group**(object_type *o_ptr) — Add a flags group — synced from report
- [x] `object1.cc:5510` **get_flag**(object_type *o_ptr, int grp) — synced from report
- [x] `object1.cc:5550` **gain_flag_group_flag**(object_type *o_ptr) — Add a flags from a flag group — synced from report
- [x] `object1.cc:5589` **object_gain_level**(object_type *o_ptr) — When an object gain a level, he can gain some attributes — synced from report
- [x] `object1.cc:5647` **wield_set**(s16b a_idx, s16b set_idx, bool silent) — Item sets fcts
- [x] `object1.cc:5683` **takeoff_set**(s16b a_idx, s16b set_idx)
- [x] `object1.cc:5719` **apply_set**(s16b a_idx, s16b set_idx)
- [x] `object1.cc:5751` **apply_flags_set**(s16b a_idx, s16b set_idx, object_flag_set *f)
- [x] `object1.cc:5778` **object_attr**(object_type const *o_ptr) — synced from report
- [x] `object1.cc:5802` **object_attr_default**(object_type const *o_ptr)
- [x] `object1.cc:5826` **object_char**(object_type const *o_ptr) — synced from report
- [x] `object1.cc:5844` **object_char_default**(object_type const *o_ptr)
- [x] `object1.cc:5865` **artifact_p**(object_type const *o_ptr) — Is the given object an artifact? — synced from report
- [x] `object1.cc:5877` **ego_item_p**(object_type const *o_ptr) — Is the given object an ego item? — synced from report
- [x] `object1.cc:5885` **is_ego_p**(object_type const *o_ptr, s16b ego) — Is the given object an ego item of the given type? — synced from report
- [x] `object1.cc:5893` **cursed_p**(object_type const *o_ptr) — Is the given object cursed? — synced from report

## object2.cc (75 defs)

- [x] `object2.cc:57` **calc_total_weight**() — Calculate the player's total inventory weight. — synced from report
- [~] `object2.cc:75` **excise_object_idx**(int o_idx) — [~] 地物/对象列表与空闲槽由 ECS FloorItem/LevelStore 承担（map.rs/game.rs），无 o_list 手工管理；accept.py object-list 类机械复核
- [~] `object2.cc:116` **delete_object_idx**(int o_idx) — [~] 地物/对象列表与空闲槽由 ECS FloorItem/LevelStore 承担（map.rs/game.rs），无 o_list 手工管理；accept.py object-list 类机械复核
- [~] `object2.cc:150` **delete_object**(int y, int x) — [~] 地物/对象列表与空闲槽由 ECS FloorItem/LevelStore 承担（map.rs/game.rs），无 o_list 手工管理；accept.py object-list 类机械复核
- [~] `object2.cc:182` **compact_objects_aux**(int i1, int i2) — [~] 地物/对象列表与空闲槽由 ECS FloorItem/LevelStore 承担（map.rs/game.rs），无 o_list 手工管理；accept.py object-list 类机械复核
- [~] `object2.cc:242` **compact_objects**(int size) — [~] 地物/对象列表与空闲槽由 ECS FloorItem/LevelStore 承担（map.rs/game.rs），无 o_list 手工管理；accept.py object-list 类机械复核
- [x] `object2.cc:372` **rescue_artifact**(object_type *o_ptr) — Rescue artifacts from destruction if the "preserve" option is turned on. — synced from report
- [~] `object2.cc:405` **wipe_o_list**() — [~] 地物/对象列表与空闲槽由 ECS FloorItem/LevelStore 承担（map.rs/game.rs），无 o_list 手工管理；accept.py object-list 类机械复核
- [~] `object2.cc:468` **o_pop**() — [~] 地物/对象列表与空闲槽由 ECS FloorItem/LevelStore 承担（map.rs/game.rs），无 o_list 手工管理；accept.py object-list 类机械复核
- [x] `object2.cc:524` **get_obj_num_prep**() — Apply a "object restriction function" to the "object allocation table" — synced from report
- [x] `object2.cc:568` **get_obj_num**(int level) — Choose an object kind that seems "appropriate" to the given level This function uses the "prob2" field of the "object allocation table", and various local information, to calculate the "prob3" field o — synced from report
- [x] `object2.cc:708` **object_known**(object_type *o_ptr) — Known is true when the "attributes" of an object are "known". These include tohit, todam, toac, cost, and pval (charges). Note that "knowing" an object gives you everything that an "awareness" gives y — synced from report
- [x] `object2.cc:729` **object_known_p**(object_type const *o_ptr) — Determine if a given inventory item is "known" Test One -- Check for special "known" tag Test Two -- Check for "Easy Know" + "Aware" — synced from report
- [x] `object2.cc:740` **object_aware**(object_type *o_ptr) — The player is now aware of the effects of the given object. — synced from report
- [x] `object2.cc:749` **object_aware_p**(object_type const *o_ptr) — Is the player aware of the effects of the given object? — synced from report
- [x] `object2.cc:756` **flag_cost**(object_type const *o_ptr, int plusses) — Return the value of the flags the object has... — synced from report
- [x] `object2.cc:990` **object_value_real**(object_type const *o_ptr) — Return the "real" price of a "known" item, not including discounts Wand and staffs get cost for each charge Armor is worth an extra 100 gold per bonus point to armor class. Weapons are worth an extra  — synced from report
- [x] `object2.cc:1300` **object_value**(object_type const *o_ptr) — Return the price of an item including plusses (and charges) This function returns the "value" of the given item (qty one) — synced from report
- [x] `object2.cc:1341` **object_similar**(object_type const *o_ptr, object_type const *j_ptr) — Determine if an item can "absorb" a second item See "object_absorb()" for the actual "absorption" code. If permitted, we allow wands/staffs (if they are known to have equal charges) and rods (if fully — synced from report
- [x] `object2.cc:1631` **object_absorb**(object_type *o_ptr, object_type *j_ptr) — Allow one item to "absorb" another, assuming they are similar — synced from report
- [x] `object2.cc:1663` **lookup_kind**(int tval, int sval) — Find the index of the object_kind with the given tval and sval — synced from report
- [~] `object2.cc:1687` **object_wipe**(object_type *o_ptr) — [~] 地物/对象列表与空闲槽由 ECS FloorItem/LevelStore 承担（map.rs/game.rs），无 o_list 手工管理；accept.py object-list 类机械复核
- [x] `object2.cc:1697` **object_copy**(object_type *o_ptr, object_type *j_ptr) — Prepare an object based on an existing object
- [x] `object2.cc:1708` **init_obj_exp**(object_type *o_ptr, std::shared_ptr<object_kind const> k_ptr) — Initialize the experience of an object which is a "sentient" object. — synced from report
- [x] `object2.cc:1718` **object_prep**(object_type *o_ptr, int k_idx) — Prepare an object based on an object kind. — synced from report
- [x] `object2.cc:1808` **m_bonus**(int max, int level) — Help determine an "enchantment bonus" for an object. To avoid floating point but still provide a smooth distribution of bonuses, we simply round the results of division in such a way as to "average" t — synced from report
- [x] `object2.cc:1856` **finalize_randart**(object_type* o_ptr, int lev) — Tinker with the random artifact to make it acceptable for a certain depth; also connect a random artifact to an object.
- [~] `object2.cc:1890` **object_mention**(object_type *o_ptr) — [~] 地物/对象列表与空闲槽由 ECS FloorItem/LevelStore 承担（map.rs/game.rs），无 o_list 手工管理；accept.py object-list 类机械复核
- [x] `object2.cc:1925` **random_artifact_power**(object_type *o_ptr)
- [x] `object2.cc:1979` **random_artifact_resistance**(object_type * o_ptr)
- [x] `object2.cc:2037` **make_artifact_special**(object_type *o_ptr) — Mega-Hack -- Attempt to create one of the "Special Objects" We are only called from "make_object()", and we assume that "apply_magic()" is called immediately after we return. Note -- see "make_artifac
- [x] `object2.cc:2120` **make_artifact**(object_type *o_ptr) — Attempt to change an object into an artifact This routine should only be called by "apply_magic()" Note -- see "make_artifact_special()" and "apply_magic()" — synced from report
- [x] `object2.cc:2192` **make_ego_item**(object_type *o_ptr, bool good) — Attempt to change an object into an ego This routine should only be called by "apply_magic()" — synced from report
- [x] `object2.cc:2337` **charge_stick**(object_type *o_ptr) — Charge a new stick.
- [x] `object2.cc:2350` **a_m_aux_1**(object_type *o_ptr, int level, int power) — Apply magic to an item known to be a "weapon" Hack -- note special base damage dice boosting Hack -- note special processing for weapon/digger Hack -- note special rating boost for dragon scale mail — synced from report
- [x] `object2.cc:2447` **dragon_resist**(object_type * o_ptr) — synced from report
- [x] `object2.cc:2468` **a_m_aux_2**(object_type *o_ptr, int level, int power) — Apply magic to an item known to be "armor" Hack -- note special processing for crown/helm Hack -- note special processing for robe of permanence — synced from report
- [x] `object2.cc:2597` **a_m_aux_3**(object_type *o_ptr, int level, int power) — Apply magic to an item known to be a "ring" or "amulet" Hack -- note special rating boost for ring of speed Hack -- note special rating boost for amulet of the magi Hack -- note special "pval boost" c — synced from report
- [x] `object2.cc:2982` **randomized_level_in_range**(range_type *range, int level) — Randomized level
- [x] `object2.cc:3003` **get_stick_base_level**(byte tval, int level, int spl) — Get a random base level
- [x] `object2.cc:3014` **get_stick_max_level**(byte tval, int level, int spl) — Get a random max level
- [x] `object2.cc:3028` **a_m_aux_4**(object_type *o_ptr, int level, int power) — Apply magic to an item known to be "boring" Hack -- note the special code for various items — synced from report
- [x] `object2.cc:3292` **add_random_ego_flag**(object_type *o_ptr, ego_flag_set const &fego, bool *limit_blows) — Add a random glag to the ego item — synced from report
- [x] `object2.cc:3724` **apply_magic**(object_type *o_ptr, int lev, bool okay, bool good, bool great, boost::optional<int> force_power) — Complete the "creation" of an object by applying "magic" to the item This includes not only rolling for random bonuses, but also putting the finishing touches on ego-items and artifacts, giving charge — synced from report
- [x] `object2.cc:4108` **init_match_theme**(obj_theme const &theme) — XXX XXX XXX It relies on the fact that obj_theme is a four byte structure for its efficient operation. A horrendous hack, I'd say. — synced from report
- [x] `object2.cc:4129` **kind_is_theme**(obj_theme const *theme, object_kind const *k_ptr) — Maga-Hack -- match certain types of object only. — synced from report
- [x] `object2.cc:4311` **kind_is_legal**(object_kind const *k_ptr) — Determine if an object must not be generated. — synced from report
- [x] `object2.cc:4354` **kind_is_good**(object_kind const *k_ptr) — Hack -- determine if a template is "good" — synced from report
- [x] `object2.cc:4445` **kind_is_artifactable**(object_kind const *k_ptr) — Determine if template is suitable for building a randart -- dsb — synced from report
- [x] `object2.cc:4484` **make_object**(object_type *j_ptr, bool good, bool great, obj_theme const &theme) — Attempt to make an object (normal or good/great) This routine plays nasty games to generate the "special artifacts". This routine uses "object_level" for the "generation level". We assume that the giv — synced from report
- [x] `object2.cc:4603` **place_object**(int y, int x, bool good, bool great, int where) — Attempt to place an object (normal or good/great) at the given location. This routine plays nasty games to generate the "special artifacts". This routine uses "object_level" for the "generation level" — synced from report
- [x] `object2.cc:4714` **make_gold**(object_type *j_ptr) — Make a treasure object The location must be a legal, clean, floor grid. — synced from report
- [x] `object2.cc:4758` **place_gold**(int y, int x) — Places a treasure (Gold or Gems) at given location The location must be a legal, clean, floor grid. — synced from report
- [x] `object2.cc:4834` **drop_near**(object_type *j_ptr, int chance, int y, int x) — Let an object fall to the ground at or near a location. The initial location is assumed to be "in_bounds()". This function takes a parameter "chance". This is the percentage chance that the item will  — synced from report
- [x] `object2.cc:5123` **acquirement**(int y1, int x1, int num, bool great) — Scatter some "great" objects near the player — synced from report
- [~] `object2.cc:5151` **inven_item_charges**(int item) — [~] 堆叠/充能簿记并入 Item::count/charges 与 items_similar/inven_item_optimize（item.rs:1227/1240），命令层直接调用；accept.py stack-bookkeeping 类机械复核
- [~] `object2.cc:5180` **inven_item_describe**(int item) — [~] 堆叠/充能簿记并入 Item::count/charges 与 items_similar/inven_item_optimize（item.rs:1227/1240），命令层直接调用；accept.py stack-bookkeeping 类机械复核
- [~] `object2.cc:5196` **inven_item_increase**(int item, int num) — [~] 堆叠/充能簿记并入 Item::count/charges 与 items_similar/inven_item_optimize（item.rs:1227/1240），命令层直接调用；accept.py stack-bookkeeping 类机械复核
- [x] `object2.cc:5234` **inven_item_optimize**(int item) — Erase an inventory slot if it has no more items — synced from report
- [~] `object2.cc:5307` **floor_item_charges**(int item) — [~] 堆叠/充能簿记并入 Item::count/charges 与 items_similar/inven_item_optimize（item.rs:1227/1240），命令层直接调用；accept.py stack-bookkeeping 类机械复核
- [~] `object2.cc:5337` **floor_item_describe**(int item) — [~] 堆叠/充能簿记并入 Item::count/charges 与 items_similar/inven_item_optimize（item.rs:1227/1240），命令层直接调用；accept.py stack-bookkeeping 类机械复核
- [~] `object2.cc:5353` **floor_item_increase**(int item, int num) — [~] 堆叠/充能簿记并入 Item::count/charges 与 items_similar/inven_item_optimize（item.rs:1227/1240），命令层直接调用；accept.py stack-bookkeeping 类机械复核
- [~] `object2.cc:5375` **floor_item_optimize**(int item) — [~] 堆叠/充能簿记并入 Item::count/charges 与 items_similar/inven_item_optimize（item.rs:1227/1240），命令层直接调用；accept.py stack-bookkeeping 类机械复核
- [~] `object2.cc:5393` **inc_stack_size**(int item, int delta) — [~] 堆叠/充能簿记并入 Item::count/charges 与 items_similar/inven_item_optimize（item.rs:1227/1240），命令层直接调用；accept.py stack-bookkeeping 类机械复核
- [~] `object2.cc:5400` **inc_stack_size_ex**(int item, int delta, optimize_flag opt, describe_flag desc) — [~] 堆叠/充能簿记并入 Item::count/charges 与 items_similar/inven_item_optimize（item.rs:1227/1240），命令层直接调用；accept.py stack-bookkeeping 类机械复核
- [x] `object2.cc:5435` **inven_carry_okay**(object_type const *o_ptr) — Check if we have space for an item in the pack without overflow — synced from report
- [x] `object2.cc:5496` **inven_carry**(object_type *o_ptr, bool final) — Add an item to the players inventory, and return the slot used. If the new item can combine with an existing item in the inventory, it will do so, using "object_similar()" and "object_absorb()", other — synced from report
- [x] `object2.cc:5653` **inven_takeoff**(int item, int amt, bool force_drop) — Take off (some of) a non-cursed equipment item Note that only one item at a time can be wielded per slot. Note that taking off an item when "full" may cause that item to fall to the ground. Return the — synced from report
- [x] `object2.cc:5761` **inven_drop**(int item, int amt, int dy, int dx, bool silent) — Drop (some of) a non-cursed inventory/equipment item The object will be dropped "near" the current location — synced from report
- [x] `object2.cc:5838` **combine_pack**() — Combine items in the pack Note special handling of the "overflow" slot — synced from report
- [x] `object2.cc:5911` **reorder_pack**() — Reorder items in the pack Note special handling of the "overflow" slot — synced from report
- [x] `object2.cc:6022` **floor_carry**(int y, int x, object_type *j_ptr) — Let the floor carry an object — synced from report
- [x] `object2.cc:6083` **pack_decay**(int item) — Notice a decaying object in the pack — synced from report
- [x] `object2.cc:6170` **floor_decay**(int item) — Decay an object on the floor — synced from report
- [~] `object2.cc:6265` **get_object**(int item) — [~] 堆叠/充能簿记并入 Item::count/charges 与 items_similar/inven_item_optimize（item.rs:1227/1240），命令层直接调用；accept.py stack-bookkeeping 类机械复核

## object_filter.cc (11 defs)

- [x] `object_filter.cc:8` **TVal**(byte tval)
- [x] `object_filter.cc:14` **SVal**(byte sval)
- [x] `object_filter.cc:20` **HasFlags**(object_flag_set const &mask)
- [x] `object_filter.cc:27` **IsArtifact**() — bevy/src/item.rs:533 is_standard_artifact（name1>0）；smith 检查 modal.rs:21062/21106 使用；测试 standard_artifact_predicate_excludes_randarts
- [x] `object_filter.cc:33` **IsArtifactP**()
- [x] `object_filter.cc:39` **IsEgo**()
- [x] `object_filter.cc:45` **IsKnown**()
- [x] `object_filter.cc:51` **True**()
- [x] `object_filter.cc:57` **Not**(object_filter_t p) — synced from report
- [x] `object_filter.cc:63` **And**()
- [x] `object_filter.cc:69` **Or**() — synced from report

## object_flag_meta.cc (0 defs)

> No defs. The metadata table (name/negated/esp/pval mask) drives C++'s
> object flag prose and `object_flags_esp()`/`compute_pval_mask()`.  Rust
> consumes ESP_* by string prefix and does not need the pval mask, but has
> no flag-name/negation table (flag names are printed raw in the observe
> window).  No missing gameplay consumer found.
