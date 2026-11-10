# Registro de cambios

[English](../CHANGELOG.md) · otros idiomas: ver el [registro en inglés](../CHANGELOG.md).

## BetterGameExperience 0.8.1

### Novedades
- **Cancelar la producción desde el estandarte de la ciudad**: un clic derecho en el icono de
  producción del estandarte de una de tus ciudades retira el elemento en curso, como en el panel de
  producción.
- **Distritos con descuento**: en la lista de producción, un distrito cuyo coste está reducido
  (tienes menos distritos de ese tipo que tu media) muestra su descuento, por ejemplo **-40%**. Se
  actualiza en cuanto se coloca un distrito, ya que colocar uno puede acabar con el descuento. Los
  distritos que no cuentan en la media, como el Thành de Vietnam, también se indican.
- **Producción en reserva**: el estandarte de la ciudad, el panel de la ciudad y el panel de
  producción muestran la producción que la ciudad guarda en reserva (excedente de un elemento
  terminado, o un turno sin producir; el juego guarda como máximo un turno), que se suma a su
  producción del turno siguiente. En el estandarte de una ciudad sin producción aparece como **+N**.

### Correcciones de la interfaz
- **Panel de la ciudad tras retirar la producción**: el panel de abajo a la derecha se actualiza al
  retirar el único elemento de la cola, en lugar de seguir mostrándolo.
- **Clic derecho en la producción actual**: ahora también la retira desde la pestaña Producción, no
  solo con la cola abierta.
- **Sala de MPH**: fuera de un draft, la lista de jugadores sube al espacio vacío que MPH reserva
  para su draft, y desaparece el « . » que aparecía encima en una partida recargada.

### Ajustes
- Las tres novedades tienen cada una un ajuste en la pestaña BetterGameExperience, activado por
  defecto.

## BetterGameExperience 0.8.0

En multijugador, todos los jugadores humanos deben tener la 0.8.0. La corrección de los acuerdos al
final del turno solo protege a quienes la tienen: un solo jugador que siga en 0.7.x puede traer de
vuelta el turno bloqueado para todos. En la sala, el anfitrión ve en rojo quién tiene otra versión, y
una sala que mezcla 0.7.x y 0.8.0 desactiva las correcciones **[todos los jugadores]**.

### Correcciones del multijugador [todos los jugadores]
- **Turno en bucle tras un acuerdo entre humanos**: un acuerdo propuesto o aceptado justo al final
  de tu turno podía llegar al anfitrión después del final del turno. El turno siguiente se reiniciaba
  entonces sin fin para algunos jugadores, había que volver a la sala y la partida solía
  desincronizarse. Un acuerdo así ya no se envía una vez agotado tu temporizador, ni cuando ningún
  humano tiene ya su turno activo: una línea en tu chat te lo indica. Los acuerdos con las IA no
  cambian.

### Novedades
- Botón **Informar de un error (BGE)** en el menú Esc, bajo Opciones. Describe el problema: BGE envía
  una partida guardada, los últimos autoguardados y los registros del juego (sin tu nombre de
  usuario de Windows), y una línea de tu chat da el número del informe. Los demás jugadores ven en el
  chat que se está enviando un informe. Si el envío falla, el zip se copia en tu Escritorio.

### Correcciones de la interfaz
- **Efecto de las cartas de política**: Racionalismo, Gran ópera, Simultaneum y Libre mercado
  cuentan también los rendimientos fijos añadidos a los edificios que potencian (gobernadores…), y
  no solo su rendimiento base. Las bonificaciones de las ciudades-estado en esos edificios quedan
  fuera, como en el juego, que no las multiplica.
- **Selección de rutas comerciales**: en una ruta interior, ahora se muestra el oro que la capacidad
  de Poundmaker da a la ciudad de destino (+1 por campamento y por pasto), con una línea en la
  descripción emergente que nombra la ciudad.
- **Barra superior**: el botón de población muestra también la media de servicios netos por ciudad
  (en verde desde +3, en rojo por debajo de 0).
- **Menú principal**: la versión de BetterGameExperience aparece abajo a la izquierda, después de la
  del juego y la de MPH (`… - MPH / … - BGE / 0.8.0`).

### Ajustes
- Opción de partida **«No First/Last moves (BGE)»**: ahora marcada por defecto. El anfitrión puede
  seguir desmarcándola.

## BetterGameExperience 0.7.0

En multijugador, las correcciones **[todos los jugadores]** requieren que todos los jugadores
humanos tengan la 0.7.0: una sala que mezcla 0.6.x y 0.7.0 las desactiva.

### Correcciones de juego [todos los jugadores]
- **Unidades apiladas**: dos unidades militares de la misma clase pueden acabar en la misma casilla
  (unidad comprada en una ciudad ocupada, movimiento interrumpido, teletransporte…). La unidad
  sobrante ya no cuenta en las bonificaciones de flanqueo y apoyo: dos unidades apiladas junto al
  objetivo dan +2, no +4. La defensa de las ciudades no cambia.

### Congreso Mundial
- **Retiradas**: las correcciones de la 0.6.x que impedían el fin del turno durante la votación (sin
  fin de turno automático, sin fin de turno forzado con una lista de resoluciones vacía, 240 s en el
  temporizador de MPH). El temporizador y el fin de turno del juego y de MPH vuelven a aplicarse
  durante la votación. El botón **Volver a la votación**, la pantalla de bienvenida una vez por
  sesión y el recálculo de los lujos se mantienen.

### Correcciones de la interfaz
- **Pantalla de ciudades-estado**: los emisarios colocados pero aún no confirmados ya no se pierden
  cuando la pantalla se actualiza (emisario de otro jugador, misión, guerra o paz). Solo se retiran
  si la ciudad-estado es destruida o si tu equipo le declara la guerra.
- **Pantalla de acuerdo**: el botón del oro disponible vuelve a funcionar. Podía quedarse sin poder
  pulsarse tras un acuerdo anterior (error del juego).
- **Mapa completo desde el inicio** (activado por defecto): el minimapa y la cámara cubren todo el
  mapa desde el primer turno, ya no hacen falta chinchetas en los bordes del mapa. Las zonas sin
  explorar siguen ocultas. Puramente local.
- **Área de movimiento de una unidad sobre un distrito enemigo**: se acabaron los tirones al
  seleccionarla (campamento enemigo sin terminar), se muestran las zonas
  de control (casillas rojas) y ya no se proponen casillas de agua a las que la unidad aún no puede
  ir (costa antes del embarque, océano antes de la Cartografía).

### Ajustes
- Botón **Restablecer BGE** en la pestaña BetterGameExperience: devuelve todos los ajustes a su
  valor por defecto.
- Nuevo ajuste avanzado de diagnóstico, desactivado por defecto: **Medir el coste del área de
  movimiento (en tiempo)**.

## BetterGameExperience 0.6.1

Incluye todo lo de la versión preliminar 0.6.0. En multijugador, las correcciones **[todos los
jugadores]** requieren que todos los jugadores humanos tengan la 0.6.0 o la 0.6.1: una sala que
mezcla 0.5.0 y 0.6.x las desactiva.

### Instalación y actualizaciones
- El instalador ya no muestra la opción de lanzamiento de Steam tras instalar: indica que todo está
  listo y puedes cerrarlo. El aviso de actualización del juego basta para estar al día. Una opción de
  lanzamiento que ya añadiste sigue funcionando.
- El instalador ahora se llama `BetterGameExperience_V<version>.exe`. Al ejecutarlo, borra los
  instaladores antiguos de la misma carpeta (normalmente Descargas).

### Multijugador
- **"No First/Last moves (BGE)"** **[todos los jugadores]**: te quedas en la pantalla de cambio de
  turno, botón de fin de turno incluido, hasta que el anfitrión inicia el turno, en lugar de ver tu
  turno abierto con tus órdenes rechazadas.

### Congreso Mundial
- *(experimental)* **[todos los jugadores]** Mientras esperas a los demás jugadores, un botón
  **Volver a la votación** reabre la ventana de votación con tus votos y la cantidad correcta de
  Favor Diplomático.
- *(experimental)* El turno ya no termina contra tu voluntad durante la votación: sin fin de turno
  automático, sin fin de turno forzado ante una lista de resoluciones vacía, y el temporizador de MPH
  da los 240 s previstos para la votación.
- *(experimental)* **[todos los jugadores]** Los lujos afectados por una prohibición se recalculan
  en cada sesión.
- La pantalla de bienvenida del Congreso Mundial y su sonido solo aparecen una vez por sesión, en
  lugar de volver cada vez que otro jugador vuelve a la votación.

### Correcciones de la interfaz
- Con Better Spectator Mod, la barra superior vuelve a mostrar los recursos estratégicos de tus
  compañeros de equipo, y los recursos que desbloquea un compañero.

## BetterGameExperience 0.5.0: primera versión pública

BetterGameExperience (BGE) corrige y acelera Civilization VI: Gathering Storm con dos DLL colocadas junto a los
archivos del juego. **No se reemplaza ningún archivo del juego**: ni la verificación de archivos de
Steam ni las actualizaciones del juego lo eliminan. Funciona con DirectX 11 y DirectX 12, con BBG,
Better Spectator Mod y MPH.

Todos los cambios se enumeran a continuación, por categoría. Los marcados **[todos los jugadores]**
cambian las reglas de la partida: solo se activan cuando todos los jugadores humanos de la sala
tienen la misma versión de BetterGameExperience. Todo lo demás solo afecta a tu equipo. Se puede jugar sin
riesgo con jugadores que no tienen el mod.

### Instalación y actualizaciones
- Instalador `BetterGameExperience.exe`: instalar, actualizar, reparar y desinstalar. Encuentra el juego en
  tus bibliotecas de Steam y comprueba el manifiesto firmado y el SHA-256 de cada archivo antes de
  copiar nada.
- Se añaden dos archivos junto a los del juego: `winmm.dll` y `BGD_GameCore_XP2.dll`. Las
  instalaciones anteriores (0.3.0 y previas) se limpian automáticamente. Nunca se sobrescribe un
  `winmm.dll` de otro mod.
- Aviso de actualización en el menú principal, también en el de MPH. **Actualizar** cierra el juego,
  instala la nueva versión y ofrece reiniciarlo.
- Casilla «Versión preliminar» en el instalador para probar las próximas versiones, con vuelta a la
  versión estable en un clic.
- Opción de lanzamiento de Steam opcional, que busca actualizaciones cada vez que inicias el juego.
- Instalador en 11 idiomas, los de Better Balanced Game. Sigue el idioma de Windows, y puedes elegir
  otro en la parte inferior de la ventana.

### Ajustes
- Nueva pestaña **BetterGameExperience** en las opciones del juego: un interruptor general que lo desactiva
  todo sin desinstalar, un ajuste por función y opciones avanzadas ocultas por defecto. Avisa cuando
  un cambio necesita reiniciar.
- Créditos para los autores de los mods de interfaz integrados.
- Todos los textos del juego están disponibles en los 11 idiomas de Better Balanced Game: inglés,
  francés, alemán, español, italiano, japonés, coreano, polaco, portugués de Brasil, ruso y chino
  simplificado.

### Multijugador
- **Comprobación en la sala**: cada equipo comunica su versión de BetterGameExperience, y el anfitrión decide
  por todos. Los mensajes de chat con el prefijo `BGE -` indican a quién le falta el mod o quién tiene
  otra versión. Se tienen en cuenta las reconexiones. Si alguien se une sin el mod mientras las
  correcciones **[todos los jugadores]** están activas, el anfitrión recibe un aviso.
- Los mensajes de chat `BGE -` se envían en inglés, para que los jugadores sin el mod puedan leerlos.
  Los jugadores con el mod los ven en el idioma de su juego.
- **Opción de partida «No First/Last moves (BGE)»** **[todos los jugadores]**:
  - Todos los jugadores empiezan su turno a la vez. Las órdenes se retienen, y el temporizador sigue
    parado, hasta que todos hayan terminado de cargar (60 s como máximo).
  - El turno termina para todos cuando se acaba el temporizador del anfitrión.
  - Se desactiva la antigua sincronización de turno de MPH.
- Partidas cargadas en la sala de MPH: las ciudades-estado ya no provocan «Opciones de jugador no
  válidas» ni el botón Listo en gris.

### Correcciones de juego [todos los jugadores]
- Casillas trabajadas: una ciudad ya no trabaja una casilla 2/1 mientras hay una 2/2 libre (enfoque
  en comida y producción).
- Los espectadores de Better Spectator Mod ya no cuentan en el turismo. No pueden votar en el Congreso
  Mundial ni ser objetivo de una resolución. Siguen pudiendo consultar la pantalla del Congreso.
- Las unidades navales pueden teletransportarse a una ciudad desde cualquier distrito naval (puerto,
  parque acuático, canal…), y no solo desde un centro urbano, igual que ya podían teletransportarse
  de una ciudad a cualquier distrito naval (se acabaron los almirantes atascados en un lago de una
  casilla).

### Correcciones de la interfaz
- Los botones de ataque (ciudad, campamento, ataque nuclear desde un silo) se dibujan sobre las
  banderas de unidades, donde se pueden pulsar.
- El ataque a distancia desde una ciudad no seleccionada requiere un clic, no dos.
- Al apuntar un ataque a distancia de ciudad o de distrito, pulsar la bandera de una unidad enemiga
  dispara contra ella.
- La cámara ya no se centra al pulsar un botón de ataque (se puede volver a activar).
- El contador de turnos de producción ya incluye las bonificaciones de gobierno: fascismo, autocracia
  y república mercantil.
- Una maravilla que sale de la cola de producción tras dos intercambios se vuelve a poner.
- La producción en curso se actualiza correctamente al pasar a otra ciudad.
- Los rendimientos del imperio en la barra superior se actualizan tras una acción de gobernador o un
  gran personaje.
- *(experimental)* Los atajos de teclado ya no dejan de funcionar cuando una pantalla de diplomacia
  cierra el menú Esc, o tras una ventana doble del Congreso.
- Esc cierra ahora correctamente la pantalla de enviados: los enviados pendientes ya no bloquean el
  final del turno.
- *(experimental)* La lente del colono se vuelve a dibujar tras fundar una ciudad, y las casillas
  demasiado cercanas a una ciudad aparecen como prohibidas.
- Intro sigue tu atajo de «Siguiente acción»: puedes quitarlo de ahí para asignarlo a otro atajo.
  Mayús+Intro sigue forzando el final del turno.
- Se muestra el área de movimiento de una unidad situada en un distrito defendible enemigo.
- *(experimental)* Los marcadores de mapa ya no se reconstruyen cuando nada ha cambiado: menos
  ralentizaciones en multijugador.

### Mods de interfaz integrados
Cada uno se puede desactivar en la pestaña BetterGameExperience. Si también tienes el mod del Workshop, la
versión integrada en BGE lo sustituye.
- **Cargas de los constructores** en las banderas de unidades (de Better Builder Charges Tracking, por
  wltk).
- **Turnos restantes de las rutas comerciales** en las banderas de tus comerciantes (nuevo).
- Botón **Borrar notificaciones** (de NotificationClear, por 336tarot).
- **Barra superior ampliada**: comida, producción, población y lujos, con detalle por ciudad (de
  TopPanel Extension Pro, por 号码菌 / Azukii).
- **Efecto de las cartas de política**: cada carta muestra su ganancia real para tu imperio (de
  Extended Policy Cards, por Aristos). Ya no hace falta Better Report Screen. Valores corregidos
  para:
  - Raj, Medina, Monacato y Prestigio civil;
  - Reclutamiento y Leva en masa;
  - las cartas Pretorio e Impuestos coloniales;
  - los puntos de grandes personajes, ahora mostrados como [icono]+x.
- **Pantalla de informes mejorada** con las pestañas Acuerdos, Unidades, Políticas, Ciudades-estado y
  Ciudades (de Better Report Screen, por Infixo), con las mismas correcciones.

### Rendimiento y comodidad
- Cambio de turno más rápido:
  - Durante una cascada de declaraciones de guerra, los datos de ciudades y diplomacia solo se
    actualizan una vez. El turno medido fue 2,4 s más rápido (−82 % en la cascada): mucho menos lag
    cuando las ciudades-estado cambian de bando en una guerra.
  - Dos cachés aceleran el turno de la IA: mejoras construibles y rendimientos de casillas.
  - Los resultados son idénticos a los del juego base: sin riesgo con jugadores que no tienen el mod.
- La animación de los ataques nucleares es 20 veces más rápida con el combate rápido activado: unos
  0,6 s en lugar de 11 s. Solo visualización.
- El limitador de fotogramas ofrece 45, 75, 90 y 100 FPS.

### Limitaciones conocidas
- El mod está hecho para la versión oficial actual del juego. Tras una actualización del juego, las
  funciones afectadas se desactivan solas hasta que BetterGameExperience se actualice.
- Un jugador que se une a mitad de partida sin el mod, mientras las correcciones **[todos los
  jugadores]** están activas, se desincroniza. El anfitrión recibe un aviso.
- Las correcciones marcadas *(experimental)* aún no se han probado en una partida real, o su efecto
  aún no se ha medido del todo.
- Las traducciones distintas del inglés y del francés aún no las ha revisado un hablante nativo. Las
  correcciones son bienvenidas.
