* add text \/
* move enemies,all stats, guns and ships etc into file for tweaking \/
* * balance enemies and player stats \+-/
* circle enemies around \easier do it with forces/
* redo gui
* * button selection
* * skills information
* * add score table

* try move camera according to gun direction


TODO LIST
* new projectiles
* * rocketes
* * granades

<!-- * Pick gun UI \/ -->
<!-- * Don't slow down bullets \/ -->
* enemies
<!-- * * rarely throws bullets around \/ -->
<!-- * * lazer beam enemy \/ -->
* * constantly trhows slow bullets in spiral(while rotating)
* * rarely throws super fast bullets
* * lazer beams around and rotating
ws bullets around
* * constantly trhows slow bullets in spiral(while rotating)
* * rarely throws super fast bullets

* Lazer add skilss
* * Lazer length

* Sample only usefull upgrades

* upgrades from files


* increase ship size with max health upgrade

* add placeholder music


<!-- * sample random skills \/ -->
<!-- * when died, restart from menu \/ -->
<!-- * skill menu via hotkey \/ -->
* blaster add skills 
* * reflection (initial speed of each reflection is less than prev)
* * additional side bullets
* * damage
* * accuracy (reduce bullets spread)
<!-- *  Angnostic skills \/ -->
<!-- * * ship rotation speed \/ -->
<!-- * * shield regen \/ -->
<!-- * * hull regen \/ -->
<!-- * * shield size \/ -->
<!-- * * hull size \/ -->

* skill choise button




not ordered by priority
<!-- * add asteroids initial movement and rotation \/
* redo asteroids explosions: fix rotation of parts when destructed \/ (seems good but when rotation is fast feels wrong?)
* redo asteroids explosions: add lifes \/ -->
* redo character asteroids collision?
<!-- * redo effects spawning (explosion when destroyed, mini explosion when shoted) \/ -->
<!-- * enemies start shoot when theay near you and stop futher \/ -->
* enemies should avoid asteroids \hacked, need something clever/
<!-- * wasd control \/ -->
* try shadows again
<!-- * lazer weapon (rotation) \/ -->
<!-- * lazer weapon (no rotation) not fit in controls X -->
<!-- * shotgun weapon \/ -->
* boosts from asteroids
 <!-- * * ship speed boost for some time X -->
<!-- * * "additional weapon"  -- trace like in bitblaster -->
* once someone got shooted make it's sprite white for a second
* add screen shake (when just shooting to?)
* ship knockback?
* sleep when killed enemy???
* * money $$$ for buying ships and weapons
* * ?shield and hull repair
* weapon rotated bullets around you like in bitblaster
* scores table
* asteroids damage?
* choose weapon and ship before start (it's saved from previous choise)
* add rotation bullets gun
* bullet trace like in nova drift looks awesome
* lazer sound
* ship sound
* animations
* explosions with animations
* add screen border markers to indicate enemies out of screen
* shotgun enemies
* * weapon modifier -- bullets reflection(for lazer it would be super cool too :D for shotgun it would be nice to add additional stike in place where reflected)
* when oriented towards speed is more and also add reactor on the back like in "weired game wich name I forgot"

* more bass to sound lol :)
* random explosions
* camera back when shooting (instead of screenshake?)
* bigger explosions 
* slow motion when dead

Skills
* Lazer
* * length
* * damage
* * reflection
* * additional side beams
* Blaster
* * initial speed
* * attack rate
* * reflection
* * additional side bullets
* * damage
* Shotgun
* * initial speed
* * attack rate
* * reflection
* * bullets life
*  Angnostic
* * ship speed
* * ship rotation speed
* * shield regen
* * hull regen
* * shield size
* * hull size


TECH AND TEST:
* Android Joystick touch controls \/

GAMEPLAY
* collectables? (from killed enemies and asteroids probably)
* enemies
* ship upgrades?
* asteroids parts

MEDIA:
* stars like here https://www.youtube.com/watch?v=s52YZMoHur0  (change background color according to how near we are)
* screen shake with depth rotation so it looks like more 3d depth shake
* sound effects
* shadows intersection bug
* thrusters
* parallax background
* particles on movement
* camera distance based on speed



Vlad Zhukov, [26.07.19 15:39]
Что-то не могу перейти left hand view матрицы к right hand (с right hand все работает, но ось y логично оказывается перепутанной из-за моего переворота в шейдере).
Делаю так:
* меняю Isometry3::look_at_rh(&observer, &target, &Vector3::y()) на Isometry3::look_at_lh(&observer, &target, &Vector3::y())
* В перспективном преобразовании меняю z_near и z_far местами, поскольку мне нужно взять его какбы с другими знаками, но оно инвариатно относительно переворота оси z: f(-z1, -z2) = f(z1, z2)
* меняю z координату observer на противоположную
* programming by permutation: пробовал отключать эти пункты по отдельности  :)))
ЧЯДНТ?


Y_INVERSE * rh_view(up = y_axis ) !=
Y_INVERSE * lh_view(up = -y_axis)

где Y_INVERSE — матрица инвертирующая y