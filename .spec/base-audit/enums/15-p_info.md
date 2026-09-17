自动生成的数据枚举清单。逐条对照 bevy 的 RON/代码标注：
`[x]`=该记录已进 RON 且其特殊行为已消费；`[>]`=进 RON 但行为部分缺失；`[ ]`=缺失；`[~]`=数据死条目/Theme/前端。

# p_info.txt — races / subraces / classes / specs

## general (5)

- [x] `general:Monster-lore` **+0:+500:Monster-lore** —  — in RON
- [x] `general:Spell-learning` **+1000:+0:Spell-learning** —  — in RON
- [x] `general:Prayer` **+0:+500:Prayer** —  — in RON
- [x] `general:Udun` **+0:+400:Udun** —  — in RON
- [x] `general:Magic-Device` **+1000:+1000:Magic-Device** —  — in RON

## races (22)

- [x] `races:0` **Human** — R:D:Humans are the second born, the Edain.;D:They are the basic race to which all others are compared.;D:Average in ability, they can be any class.;S:0:0:0:0:0:0:0;P:10:100:0;E:1:1:1:2:1:1;C:Archer;C:Loremaster;C:Mage;C:Priest;C:Rogue;C:Warrior — in RON
- [x] `races:1` **Half-Elf** — R:D:A crossbreed of elf and human, they get the best of the two races.;S:0:1:1:1:-1:1:0;P:9:110:2;E:1:1:1:2:1:1;C:Archer;C:Loremaster;C:Mage;C:Priest;C:Rogue;C:Warrior;G:ELF;k:+300:+000:Magic-Device;k:+1500:+000:Spirituality;k:+1000:+000:Stealth;k:-1… — in RON
- [x] `races:2` **Elf** — R:D:Elves are the first born, the Eldar.;D:More spiritual than physical beings, they are weaker than humans;D:but are more intelligent.;S:-1:2:2:1:-2:2:0;P:8:120:3;E:1:1:1:2:1:1;R:1:0;F:RES_LITE;C:Archer;C:Loremaster;C:Mage;C:Priest;C:Warrior;G:ELF;k… — in RON
- [x] `races:3` **Hobbit** — R:D:An old but quiet race related to humans.;D:They are small and quite weak but good at many things.;S:-2:2:1:3:2:1:5;P:7:110:4;E:1:1:1:2:1:1;Z:create food;G:RESIST_BLACK_BREATH;G:XTRA_MIGHT_SLING;R:1:0;F:SUST_DEX;C:Archer;C:Loremaster;C:Mage;C:Rogu… — in RON
- [x] `races:4` **Gnome** — R:D:Related to dwarves, Gnomes are between Dwarves and Hobbits in size.;D:Very good at magic use, they are poor as fighters.;S:-1:2:0:2:1:-2:2;P:8:135:4;E:1:1:1:2:1:1;Z:blink;R:1:0;F:FREE_ACT;C:Mage;C:Rogue;C:Warrior;k:+1200:+000:Magic-Device;k:+6000… — in RON
- [x] `races:5` **Dwarf** — R:D:The children of Aule, a strong but small race.;D:Miners and fighters of legend.;S:2:-2:2:-2:2:-3:0;P:11:125:5;E:1:1:1:2:1:1;Z:find secret passages;R:1:0;F:RES_BLIND;C:Priest;C:Warrior;k:+0:+200:Axe-mastery;k:+900:+000:Magic-Device;k:+5000:+000:Sp… — in RON
- [x] `races:6` **Orc** — R:D:Quite strong but not very smart.;S:2:-1:0:1:1:-4:-3;P:10:110:3;E:1:1:1:2:1:1;Z:remove fear;R:1:0;F:RES_DARK;C:Archer;C:Priest;C:Rogue;C:Warrior;k:-300:+000:Magic-Device;k:-1000:+000:Spirituality;k:-1000:+000:Stealth;k:+1200:+000:Weaponmastery;k:-… — in RON
- [x] `races:7` **Troll** — R:D:They can bear the light of the sun.;D:They are extremely strong and dumb.;S:4:-4:-2:-4:3:-6:-4;P:12:137:3;E:1:1:1:2:1:1;Z:berserk;R:1:0;F:SUST_STR;R:15:0;F:REGEN;C:Warrior;k:-800:+000:Magic-Device;k:-4000:+000:Spirituality;k:-2000:+000:Stealth;k:… — in RON
- [x] `races:8` **Dunadan** — R:D:The greatest of the Edain, humans in all respects but;D:stronger, smarter and wiser.;S:1:2:2:2:3:2:2;P:10:180:0;E:1:1:1:2:1:1;R:1:0;F:REGEN;F:SUST_CON;C:Archer;C:Loremaster;C:Mage;C:Priest;C:Rogue;C:Warrior;k:+500:+000:Magic-Device;k:+2500:+000:S… — in RON
- [x] `races:9` **High-Elf** — R:D:Elves are the first born, the Eldar.;D:High elves are the best of the Eldar, strong, fast, intellectual, though;D:they sometimes lack wisdom.;S:1:3:2:3:1:5:0;P:10:200:4;E:1:1:1:2:1:1;R:1:0;F:RES_LITE;F:SEE_INVIS;G:ELF;C:Archer;C:Loremaster;C:Mage… — in RON
- [x] `races:10` **Half-Ogre** — R:D:A crossbreed between a human and an ogre.;D:They are similar to half-trolls, strong and dumb.;S:3:-1:-1:-1:3:-3:-2;P:12:130:3;E:1:1:1:2:1:1;Z:set explosive rune;R:1:0;F:RES_DARK;F:SUST_STR;C:Priest;C:Warrior;k:-500:+000:Magic-Device;k:-2500:+000:… — in RON
- [x] `races:11` **Beorning** — R:D:A race of men shapeshifters.;D:They have the unique power of being able to polymorph to bear forms.;S:4:-2:-2:-1:3:-5:1;P:12:150:3;E:1:1:1:2:1:1;Z:turn into a bear;R:1:0;F:SUST_STR;R:20:1;F:STR;C:Loremaster;C:Rogue;C:Warrior;k:+1000:+1000:Bearfor… — in RON
- [x] `races:12` **Kobold** — R:D:A weaker kind of goblin, related to orcs.;S:1:-1:0:1:0:-4:0;P:9:125:3;E:1:1:1:2:1:1;Z:poison dart;R:1:0;F:RES_POIS;C:Archer;C:Rogue;C:Warrior;k:-300:+000:Magic-Device;k:-1000:+000:Spirituality;k:-1000:+000:Stealth;k:+1000:+000:Weaponmastery;k:-80… — in RON
- [x] `races:13` **Petty-Dwarf** — R:D:A nearly extinct subrace of dwarves.;D:They prefer to live in the darkness.;S:1:-1:2:0:2:-4:-5;P:11:135:5;E:1:1:1:2:1:1;Z:detect doors and traps;R:1:0;F:RES_DARK;F:RES_DISEN;C:Rogue;C:Warrior;k:+500:+000:Magic-Device;k:+5000:+000:Spirituality;k:+… — in RON
- [x] `races:14` **Dark-Elf** — R:D:Elves are the first born, the Eldar.;D:Dark elves are rare on Middle-earth and even though not evil;D:they are not good.;S:-1:3:2:2:-2:1:-2;P:9:150:5;E:1:1:1:2:1:1;Z:magic missile;R:1:0;F:RES_DARK;R:20:0;F:SEE_INVIS;C:Archer;C:Mage;C:Priest;C:Rog… — in RON
- [x] `races:15` **Ent** — R:D:Guardian of the forests of Middle-earth, summoned by Yavanna before;D:even the elves awoke. It is said 'Trolls are strong, Ents are STRONGER'.;S:10:-3:2:-5:11:-3:-2;P:14:210:5;E:1:1:1:2:1:1;Z:grow trees;G:AC_LEVEL;G:NO_FOOD;G:NO_STUN;R:1:-5;F:SEN… — in RON
- [x] `races:16` **RohanKnight** — R:D:Humans from the land of Rohan, riding the great Mearas.;D:Fast and powerful in battle.;S:4:-2:3:1:4:2:0;P:10:220:0;E:1:1:1:2:1:1;Z:Rohan Knight's Powers;R:1:3;F:SPEED;R:5:1;F:SPEED;R:10:1;F:SPEED;R:15:1;F:SPEED;R:20:1;F:SPEED;R:25:1;F:SPEED;R:30:… — in RON
- [x] `races:17` **Thunderlord** — R:D:A thunderlord is a Great Eagle of Manwe, ridden by a Maia of Manwe.;D:They carry the power of wind and thunder.;S:6:2:1:1:3:8:2;P:12:400:0;E:1:1:1:2:1:1;Z:Thunderlord's Powers;R:1:0;F:FEATHER;R:4:0;F:ESP_DRAGON;R:5:0;F:RES_ELEC;R:10:0;F:RES_COLD;… — in RON
- [x] `races:18` **DeathMold** — R:D:A pure mass of evilness, DeathMolds cannot move, but they have much more;D:power than an average race.;S:10:0:10:0:10:-15:-5;P:15:250:10;E:1:1:1:4:0:0;Z:Death Mold's Powers;G:EXPERIMENTAL;R:1:0;F:HOLD_LIFE;F:IMMOVABLE;F:RES_NETHER;F:RES_NEXUS;C:M… — in RON
- [x] `races:19` **Yeek** — R:D:The weakest of all the races, bad at everything except gaining levels quickly.;S:-5:-5:-5:-5:-5:-5:-5;P:6:25:2;E:1:1:1:2:1:1;C:Archer;C:Loremaster;C:Mage;C:Priest;C:Rogue;C:Warrior;k:-500:+000:Magic-Device;k:-2500:+000:Spirituality;k:-5000:+000:S… — in RON
- [x] `races:20` **Wood-Elf** — R:D:Elves are the first born, the Eldar.;D:Wood elves live in the great forests of Middle-earth.;S:-3:2:1:5:-4:1:0;P:7:130:4;E:1:1:1:2:1:1;G:XTRA_MIGHT_BOW;R:1:1;F:RES_LITE;F:XTRA_MIGHT;C:Archer;C:Loremaster;C:Mage;C:Priest;C:Warrior;G:ELF;k:+0:+200:… — in RON
- [x] `races:21` **Maia** — I: R:D:An old race, dating from before the creation of Arda, the Maiar were;D:created by Eru to help the Valar in their task.;S:0:0:0:0:0:0:4;P:10:100:0;E:1:1:1:2:1:1;G:NO_GOD;R:1:0;F:AGGRAVATE;R:20:0;F:DRAIN_EXP;R:6:1;F:CHR;F:CON;F:DEX;F:INT;F:STR;F… — in RON

## subraces (9)

- [x] `subraces:0` ** ** — S:D:A:A normal member of the race.;S:0:0:0:0:0:0:0:100;P:0:0:0;E:0:0:0:0:0:0;A:Beorning;A:Dark-Elf;A:DeathMold;A:Dunadan;A:Dwarf;A:Elf;A:Ent;A:Gnome;A:Half-Elf;A:Half-Ogre;A:High-Elf;A:Hobbit;A:Human;A:Kobold;A:Maia;A:Orc;A:Petty-Dwarf;A:RohanKnight;… — in RON
- [x] `subraces:1` **Vampire** — S:D:B:Vampires are powerful undead, wielding great powers. They still fear the;D:B:sunlight and cannot easily satiate their hunger.;S:0:0:0:0:0:0:0:100;P:0:0:0;E:0:0:0:0:0:0;A:Beorning;A:Dark-Elf;A:Dunadan;A:Dwarf;A:Gnome;A:Half-Elf;A:Half-Ogre;A:Hob… — in RON
- [x] `subraces:2` **Spectre** — S:D:B:Spectres only partially exist in the mortal world and so they can;D:B:pass through walls. They are somewhat physically weak.;S:-5:1:1:2:-3:-6:-3:105;P:-4:80:3;E:0:0:0:0:0:0;A:Beorning;A:Dark-Elf;A:Dunadan;A:Dwarf;A:Elf;A:Gnome;A:Half-Elf;A:Half… — in RON
- [x] `subraces:3` **Skeleton** — S:D:B:Yet an other kind of undead. Their physical 'body' is not very vulnerable;D:B:to sharp things.;S:0:-2:-2:0:1:-4:-3:70;P:0:45:1;E:0:0:0:0:0:0;A:Beorning;A:Dark-Elf;A:Dunadan;A:Dwarf;A:Elf;A:Gnome;A:Half-Elf;A:Half-Ogre;A:High-Elf;A:Hobbit;A:Huma… — in RON
- [x] `subraces:4` **Zombie** — S:D:B:Strong and dumb is a zombie.;S:2:-6:-6:1:4:-5:-4:70;P:3:45:1;E:0:0:0:0:0:0;A:Beorning;A:Dark-Elf;A:Dunadan;A:Dwarf;A:Elf;A:Gnome;A:Half-Elf;A:Half-Ogre;A:High-Elf;A:Hobbit;A:Human;A:Kobold;A:Orc;A:Petty-Dwarf;A:RohanKnight;A:Troll;A:Wood-Elf;A:… — in RON
- [x] `subraces:5` **Barbarian** — S:D:A:Hardy members of their race, they are strong fighters but poor spellcasters.;S:2:-3:-2:1:1:-3:1:50;P:1:25:0;E:0:0:0:0:0:0;A:Beorning;A:Dwarf;A:Half-Ogre;A:Human;A:Orc;A:Troll;C:F:Mage;R:10:0;F:RES_FEAR;k:-1000:+000:Magic-Device;k:+200:+000:Spir… — in RON
- [x] `subraces:6` **Hermit** — S:D:A:Through years of isolation hermits can manage to increase their mana;D:A:reserves but at the cost of an increased physical weakness.;S:-3:1:1:-3:-3:1:0:120;P:-3:20:1;E:0:0:0:0:0:0;A:Dark-Elf;A:DeathMold;A:Dunadan;A:Dwarf;A:Elf;A:Ent;A:Gnome;A:H… — in RON
- [x] `subraces:8` **LostSoul** — S:D:A:In some very rare occasions souls can come back from the Halls of Mandos.;S:0:0:0:0:0:0:0:100;P:0:0:0;E:0:0:0:0:0:0;G:ASTRAL;G:NO_SUBRACE_CHANGE;R:1:0;F:SEE_INVIS;A:Beorning;A:Dark-Elf;A:DeathMold;A:Dunadan;A:Dwarf;A:Elf;A:Ent;A:Gnome;A:Half-El… — in RON
- [x] `subraces:9` **xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx** — S:D:A:xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx;S:0:0:0:0:0:0:0:100;P:0:0:0;E:0:0:0:0:0:0 — in RON

## classes (6)

- [x] `classes:0` **0:Warrior** — C:D:0:Simple fighters, they hack away with their trusty weapon.;D:1:Rookie;D:1:Soldier;D:1:Mercenary;D:1:Veteran;D:1:Swordsman;D:1:Champion;D:1:Hero;D:1:Baron;D:1:Duke;D:1:Lord;S:5:-2:-2:2:2:-1:0:0;B:4:30:5;P:9:0;R:30:0;F:RES_FEAR;E:0:0:0:0:0:0;O:45:… — in RON
- [x] `classes:1` **3:Mage** — C:D:0:The basic spellcaster with lots of different skills;D:1:Apprentice;D:1:Trickster;D:1:Illusionist;D:1:Spellbinder;D:1:Evoker;D:1:Conjurer;D:1:Warlock;D:1:Sorcerer;D:1:Ipsissimus;D:1:Archimage;S:-5:3:0:1:-2:1:50:0;B:4:40:2;P:0:30;E:0:0:0:0:0:0;k:… — in RON
- [x] `classes:2` **1:Archer** — C:D:0:'Kill them before they see you' could be the motto of the archer class.;D:0:As deadly with a bow as a warrior is with a sword.;D:1:Rock Thrower;D:1:Slinger;D:1:Great Slinger;D:1:Tosser;D:1:Bowman;D:1:Great Bowman;D:1:Great Bowman;D:1:Archer;D:1… — in RON
- [x] `classes:3` **2:Rogue** — C:D:0:Rogues are masters of tricks. They can steal from shops and monsters,;D:0:and excel at stealthily exploring the dungeon.;D:1:Cutpurse;D:1:Robber;D:1:Burglar;D:1:Filcher;D:1:Sharper;D:1:Low Thief;D:1:High Thief;D:1:Master Thief;D:1:Assassin;D:1:… — in RON
- [x] `classes:4` **5:Loremaster** — C:D:0:Loremasters are skilled in most combat and monster skills.;D:1:Apprentice;D:1:Apprentice;D:1:Initiate;D:1:Initiate;D:1:Sage;D:1:Sage;D:1:Lorekeeper;D:1:Lorekeeper;D:1:Loremaster;D:1:Loremaster;S:1:-2:1:1:0:1:0:0;B:4:30:3;P:8:40;E:0:0:0:0:0:0;k:… — in RON
- [x] `classes:5` **4:Priest** — C:D:0:A priest serves a god (Vala, Maia or Eru himself) to bring down;D:0:the empire of fear and shadows of Morgoth.;D:1:Believer;D:1:Acolyte;D:1:Adept;D:1:Curate;D:1:Canon;D:1:Priest;D:1:High Priest;D:1:Cardinal;D:1:Inquisitor;D:1:Pope;S:-1:-3:3:-1:… — in RON

## specs (0)


