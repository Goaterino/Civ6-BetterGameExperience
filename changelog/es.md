# Registro de cambios

[English](../CHANGELOG.md) · otros idiomas: ver el [registro en inglés](../CHANGELOG.md).

## BetterGameExperience 0.14.0

En multijugador, todos los jugadores humanos deben tener la 0.14.0 para las correcciones
**[todos los jugadores]**: una sala que mezcla 0.13.0 y 0.14.0 las desactiva.

### Instalación y actualizaciones
- **Archivos firmados**: el instalador y las dos DLL de BetterGameExperience llevan ahora un
  certificado de firma de código (Certum, desarrollador de código abierto). Windows muestra el editor
  en las propiedades del archivo y en el aviso del instalador, y las alertas de SmartScreen y de los
  antivirus deberían ser menos frecuentes.

### Versiones del juego
- **Civilization VI 1.0.12.58**: BetterGameExperience reconoce ahora esta versión antigua del juego,
  en DX12 y DX11. Cada versión del juego usa sus propias direcciones, y un juego cuyos archivos no
  corresponden a su versión no instala ninguna corrección.
- **Versión del juego no compatible**: un jugador cuya versión del juego no es compatible con
  BetterGameExperience se anuncia en el chat al llegar (el mensaje decía que su copia no era la
  versión actual de Steam).

### Correcciones multijugador [todos los jugadores]
- **Menos desincronizaciones tras una recarga**: una partida cargada en una máquina calculaba de forma
  algo distinta a la partida en curso en las demás. Las mejoras recordadas de los jugadores se
  reescribían al cargar, los oyentes de eventos se ejecutaban en otro orden, la IA conservaba para
  siempre las preferencias de su líder (Sejong: -75 % de puntos de Gran Profeta) y las unidades que
  se dan al crear un distrito (bonificaciones de dificultad de la IA, inicios en eras avanzadas) se
  creaban en otro orden. Una partida cargada calcula ahora como una partida en curso. Contribución de
  OzrielTheLost.
- **Los mismos cálculos en todos los procesadores**: algunos cálculos (exponenciales, potencias)
  podían redondear de forma distinta en procesadores con y sin instrucciones FMA3. Todas las máquinas
  los calculan ahora igual. Contribución de OzrielTheLost.
- **Intercambio de unidades**: intercambiar dos unidades podía dejar una a medio camino, apilada en
  una casilla con otra unidad (un espadachín sobre un tigre agazapado). Las dos unidades terminan
  ahora su movimiento.

### Multijugador
- **Desincronizaciones reparadas sin recarga**: cuando un objeto sigue desincronizado, el anfitrión
  envía ahora todos sus datos, y la partida del jugador lo repara en el sitio en lugar de desviarse
  hasta una recarga completa. Para esto solo el anfitrión necesita la 0.14.0. Contribución de
  OzrielTheLost.

### Congreso Mundial
- **Turno del congreso tras un fin de turno forzado**: cuando el fin de turno simultáneo forzaba el
  fin del turno de un jugador, el turno del congreso podía terminar dos segundos después de abrirse,
  sin votación. El turno del congreso empieza ahora con el temporizador completo, la ventana de
  votación se vuelve a abrir si el turno terminó sin votar y el botón del congreso muestra la sesión
  actual.

### Cargas más rápidas
- **Recargas más rápidas**: con los mismos mods, una recarga conserva el paso de configuración y deja
  los textos de los mods como están, la última fase de la carga ya no espera entre sus pasos y la
  espera de la VSync se quita también en DX11. Medido en un mapa pequeño: de 13 s a 9 s. Ajustes en
  las opciones avanzadas, activados por defecto. Contribución de OzrielTheLost.

### Interfaz
- **Atajos de la pantalla de acuerdo**: en la columna «disponible», Mayús+clic añade tanto como acepta
  el acuerdo y Ctrl+clic añade diez veces un clic, para recursos, oro, oro por turno y favor.
  Activado por defecto. Contribución de huchenlei.
- **Insignia de descuento de distritos**: tras colocar un distrito con descuento en una ciudad, otra
  ciudad ya consultada seguía mostrando el descuento aunque costaba el precio completo.
- **Producción sobrante**: el «+N» del estandarte de la ciudad seguía visible después de que el turno
  lo hubiera gastado.

### Ajustes
- **Novedades de interfaz de las actualizaciones**: un ajuste nuevo al principio de la sección
  Interfaz. En «Desactivado», las novedades de interfaz de las actualizaciones siguientes llegan
  desactivadas, y las que ya tienes conservan su estado actual.

## BetterGameExperience 0.13.0

En multijugador, todos los jugadores humanos deben tener la 0.13.0 para las correcciones
**[todos los jugadores]**: una sala que mezcla 0.12.0 y 0.13.0 las desactiva.

### Actualización del juego 1.0.12.74
- **Actualización de Civilization VI del 19 de noviembre (1.0.12.74)**: BetterGameExperience ya
  reconoce el juego actualizado, en DX12 y DX11. Las versiones anteriores no instalan en él ninguna
  de sus correcciones. Contribución de yodha.

### Correcciones multijugador [todos los jugadores]
- **Jugador que se desconecta**: el juego entregaba a la IA un jugador desconectado en cada máquina
  por separado, sin ningún mensaje entre ellas, así que las máquinas podían jugar ese turno de forma
  distinta y desincronizarse. Ahora el puesto sigue siendo humano, y el anfitrión termina el turno del
  jugador ausente cuando todos los humanos conectados han terminado el suyo. Un jugador expulsado
  sigue pasando a la IA. Contribución de yodha.
- **Recarga de todos**: cuando la partida de un jugador deja de coincidir, el anfitrión inicia ahora
  la recarga y todos los jugadores cargan la misma copia de su partida; el turno solo empieza cuando
  cada jugador la ha cargado y su partida se ha comprobado. Un jugador que sigue difiriendo se recarga
  una vez más. Contribución de yodha.

## BetterGameExperience 0.12.0

En multijugador, todos los jugadores humanos deben tener la 0.12.0 para las correcciones
**[todos los jugadores]**: una sala que mezcla 0.11.0 y 0.12.0 las desactiva.

### Correcciones multijugador [todos los jugadores]
- **Desincronización tras una reconexión**: un jugador que se reconectaba, o que se recargaba solo,
  ejecutaba una partida recién cargada frente a las partidas en curso de los demás. Ambas difieren
  en datos que la comprobación de sincronización no cubre, así que la partida volvía a desviarse y
  recargaba a ese jugador una y otra vez. Ahora, cuando la partida de un jugador deja de coincidir,
  el anfitrión explica el motivo en el chat y todos recargan de inmediato la partida del anfitrión;
  el anfitrión espera la comprobación de cada jugador sobre la partida recargada y luego el turno
  empieza para todos. Contribución de yodha.
- **Reconexión al final de un turno**: un jugador que se reconectaba durante un final de turno se
  comparaba con el turno siguiente, lo que provocaba una reparación inútil y luego una recarga cada
  dos turnos. Contribución de yodha.
- **Remap de Multiplayer Helper**: tras un remap, el anfitrión ejecutaba la partida que acababa de
  generar y todos los demás jugadores la misma partida cargada desde la partida guardada del
  anfitrión, y la IA tomaba decisiones ligeramente distintas en cada lado (reparaciones de varios MB
  por turno en una partida de 8 jugadores). Ahora el anfitrión también carga su propia partida
  guardada, así que todas las máquinas ejecutan la misma partida. Coste: una pantalla de carga más
  para el anfitrión, una vez por remap. Contribución de yodha.
- **Cada partida empieza limpia**: algunos contadores del juego pasaban de una partida a la
  siguiente en la misma sesión, y los objetos se numeraban de forma distinta que en los demás
  jugadores. Contribución de yodha.

### Multijugador
- **Doble carga al iniciar una partida guardada**: el anfitrión ya no envía una segunda copia de la
  partida a los jugadores que siguen cargando, lo que hacía que algunos cargaran dos veces. Cuando
  aún se rechaza una copia, el chat muestra «Doble carga evitada». Contribución de yodha.
- **Mensajes de red dañados**: un mensaje que borraría los ajustes de la partida ahora también se
  rechaza. El chat nombra al jugador y lo que se evitó («Congelación de la partida evitada», «Borrado
  de los ajustes de la partida evitado»). Contribución de yodha.
- **Mensajes cortados al final de una carga**: un mensaje en espera durante una carga ya no se pega
  a un mensaje que todavía llega por partes. Contribución de yodha.
- **Correcciones de BetterGameExperience decididas en la carga**: la decisión de activar las
  correcciones **[todos los jugadores]** se vuelve a comprobar en la carga y al cerrarse la pantalla
  de carga, un nuevo anfitrión tras una migración la asume (los jugadores que entraban después de una
  migración jugaban sin ellas), y los jugadores ignorados siguen ignorados tras una recarga.
  Contribución de yodha.
- **Copia del juego desactualizada**: un jugador cuyo juego no es la versión actual de Steam se
  anuncia en el chat al llegar, ya que las correcciones de red de BetterGameExperience no pueden
  funcionar en su PC.
- **Ventana «Partida en pausa»**: ya no se acumula durante las cargas ni aparece sobre la pantalla
  de carga.
- **Configuración de la partida**: los avisos de equipo ya no se pueden activar ni desactivar una vez
  empezada la partida (el cambio no hacía nada o dejaba los avisos activos), partidas guardadas
  incluidas. No First/Last moves y los avisos de equipo ya no se pueden activar en una sala donde un
  jugador no tiene BetterGameExperience o la versión correcta, pero aún se pueden desactivar.
  Contribución de mariusbw.

### No First/Last moves
- La opción sigue activa una vez activada. El anfitrión ya no espera a un jugador que no estuvo
  listo dos turnos seguidos, y lo dice en el chat. Un jugador que no recibe la señal de inicio del
  anfitrión durante 75 s empieza su turno solo. Contribución de yodha.
- **Chat más discreto**: el mensaje de activación se reduce a «No First/Last moves activo.», y las
  líneas de órdenes retenidas y de salida aparecen una vez por partida en lugar de en cada turno. El
  aviso de que BetterGameExperience está activo se envía una vez por sala, no cada vez que se vuelve
  a abrir la pantalla de la sala.

### Cargas más rápidas
- Nuevos ajustes en la pestaña BetterGameExperience, todos activados por defecto, que acortan las
  cargas sin cambiar la partida: búsqueda más rápida de los archivos gráficos (varios segundos desde
  el menú principal), base de datos de juego y textos de los mods conservados entre cargas cuando los
  mods son los mismos (unos 1,5 s cada uno), sin espera de VSync en las pantallas de carga (unos 2 s,
  solo con VSync activado), una sola limpieza de Lua al cerrar la interfaz (unos 2 s antes de un
  remap), una primera carga más ligera para el anfitrión en un remap (unos 10 s) y un análisis de
  cuellos de botella más rápido al generar el mapa (unos 2 s, mismo resultado). El registro de
  cuellos de botella de la IA (AI_ChokePoint.csv, 12 MB) ya no se escribe por defecto.
- **Hilos de tareas**: los hilos inactivos desperdician menos tiempo de procesador mientras esperan
  (ajustes en la misma pestaña).

### Interfaz
- **Formación más alta primero**: en la lista de producción, el botón principal de una unidad
  entrena la formación más alta que la ciudad puede entrenar (ejército, si no cuerpo; en el mar,
  armada, si no flota), y la unidad sola pasa al menú desplegable de debajo. Las compras con oro y fe
  no cambian. Activado por defecto. Contribución de huchenlei.
- **Aviso anticipado de alojamiento**: los estandartes de tus ciudades muestran una casa naranja
  cuando a una ciudad le falta un alojamiento para el límite (crecimiento -50 %) y la casa roja en
  cuanto lo alcanza (-75 %); el juego solo mostraba la casa roja por encima del límite. Activado por
  defecto. Contribución de huchenlei.
- **Cartas de política**: sus efectos ya no desaparecen el resto de la partida cuando la pantalla de
  gobierno se dibuja por primera vez en un mal momento (visto tras recargar una partida); solo
  volvían tras otra recarga.
- **Banderas de unidades**: las unidades de combate con cargas (legión romana, toa, nao…) vuelven a
  mostrar sus ascensos en lugar de sus cargas; los constructores, los ingenieros militares y las
  demás unidades civiles y de apoyo siguen mostrando sus cargas.

### Informe de error
- Los registros que contienen tokens de inicio de sesión (FiraxisLive.log, EOS.log) se excluyen, y
  los registros largos conservan su principio y su final. Contribución de yodha.

## BetterGameExperience 0.11.0

En multijugador, todos los jugadores humanos deben tener la 0.11.0 para las correcciones
**[todos los jugadores]**: una sala que mezcla 0.10.0 y 0.11.0 las desactiva.

### Correcciones multijugador [todos los jugadores]
- **Bonificaciones de producción de unidades que se desgastan**: quitar una bonificación porcentual
  a la producción de unidades (el +30 % a los constructores de Ilkum, o la bonificación de una
  ciudad a un tipo de unidad) restaba un poco más de lo que había sumado, alrededor de un 0,4 % cada
  vez, y la pérdida crecía con cada cambio de la carta. Quitar una bonificación ahora resta
  exactamente lo que sumó. Las partidas guardadas conservan la pérdida ya acumulada. Contribución de
  Serianol.

### Multijugador
- **Partida congelada justo después de cargar**: un jugador con una conexión lenta podía recibir la
  partida por segunda vez del anfitrión justo después de terminar su carga, volver a cargarla desde
  el mapa y quedarse congelado. Esa segunda copia tardía ahora se rechaza; la entrada, la reconexión
  y las resincronizaciones funcionan como antes.
- **Cierre al cargar una partida guardada después de otra partida**: la reconexión de los jugadores
  retransmitidos por el anfitrión (0.10.0) podía leer la conexión de la partida anterior, ya cerrada,
  y cerrar el juego mientras se cargaba la siguiente. Contribución de yodha.
- **Configuración de la partida**: las opciones de partida añadidas por BetterGameExperience aparecen
  en gris cuando el anfitrión tiene BetterGameExperience desactivado.

### Señales de equipo
- Nuevas imágenes y sonidos para la rueda, los marcadores y cada llamada, instalados con
  BetterGameExperience. Solo se añaden a una partida si el anfitrión activa las señales de equipo y
  todos los jugadores tienen BetterGameExperience: un jugador sin BGE puede seguir uniéndose a la
  sala. Contribución de mariusbw.
- El **volumen de las señales de equipo** está justo debajo de los efectos de sonido en las opciones
  de audio, tanto en el menú principal como en partida, y los atajos de las señales siguen
  apareciendo en las opciones de teclas. Contribución de mariusbw.

### Ajustes
- **Pestaña BetterGameExperience**: activar o desactivar BetterGameExperience siempre pide reiniciar
  el juego. Volver al valor anterior de un ajuste sin reabrir la pantalla se guarda de nuevo; antes
  se ignoraba.

## BetterGameExperience 0.10.0

En multijugador, todos los jugadores humanos deben tener la 0.10.0 para las correcciones
**[todos los jugadores]** y las señales de equipo: una sala que mezcla 0.9.x y 0.10.0 las desactiva.

### Correcciones multijugador [todos los jugadores]
- **Jugadores bloqueados en « no listo »**: una acción diplomática (embajada, acuerdo) recibida
  justo al final del turno podía hacer que el anfitrión empezara el turno siguiente demasiado pronto.
  Algunos jugadores lo empezaban entonces ya marcados como terminados y no podían volver a estar
  listos. El turno siguiente espera ahora la comprobación del juego, y la acción se sigue aplicando.
- **Espectadores (Better Spectator Mod)**: la revisión de los espectadores de la 0.9.0 queda
  suspendida hasta probarla más a fondo, porque las partidas con un espectador eran menos estables.
  BetterGameExperience ya no cambia el nivel de civilización de los espectadores ni reemplaza los
  scripts de Better Spectator Mod, y el ajuste « Better Spectator Mod para espectadores » desaparece
  por ahora. Los espectadores vuelven a contar como civilizaciones principales para los scripts, las
  victorias y la puntuación, como con Better Spectator Mod solo. El turismo y las votaciones del
  Congreso Mundial siguen sin tenerlos en cuenta.
- **Se mantiene para los espectadores**: cambiar el jugador observado sigue sin desincronizar la
  partida, y el turno del espectador sigue terminando solo tras una pausa de MPH. Ambas correcciones
  se aplican ahora a los scripts originales de Better Spectator Mod.

### Novedades
- **Señales de equipo (BGE)**: marca una casilla para tu equipo con **X**, o mantén **X** para una
  rueda de avisos (peligro, voy en camino, ayuda, enemigo desaparecido); **Alt+1–4** los envían
  directamente. Los compañeros ven y oyen la señal con el nombre de quien la envía, los espectadores
  ven ambos equipos. Desactivado por defecto: el anfitrión marca **Señales de equipo (BGE)** en la
  configuración de la partida. Atajos en las opciones de teclas del juego, volumen en las opciones de
  audio; `/mute nombre` bloquea el chat y las señales de un jugador. Contribución de mariusbw.

### Multijugador
- **Jugadores retransmitidos por el anfitrión**: dos jugadores cuya conexión directa falló al unirse
  pasan por el anfitrión el resto de la partida, y ambos se desconectan si el anfitrión se va. Durante
  el inicio de turno simultáneo, BetterGameExperience intenta ahora conectarlos de nuevo directamente
  (tres intentos como máximo) y lo anuncia en el chat. Contribución de yodha.
- **Pings bloqueados en 0 ms**: el ping de un jugador en la lista de jugadores ya no se queda en 0 ms
  tras unirse; se vuelve a medir cada 5 segundos. Ya no hace falta salir y volver para « arreglarlo »,
  lo que a menudo provocaba una desincronización. Contribución de yodha.
- **Mensajes de red dañados**: la protección añadida en la 0.9.0 funciona también con la versión
  DirectX 11 del juego. Contribución de yodha.
- **Jugadores sin BetterGameExperience**: el aviso en partida sobre un jugador que se unió sin
  BetterGameExperience ya no se detiene por un error de script. Contribución de yodha.

### Instalación y actualizaciones
- El instalador descarga y relanza el juego sin iniciar programas auxiliares ocultos, y lleva
  información de versión: los antivirus deberían marcarlo con menos frecuencia.

## BetterGameExperience 0.9.1

Compatible con la 0.9.0 en multijugador: jugadores con 0.9.0 y 0.9.1 juegan juntos con todas las
correcciones.

### Correcciones de la interfaz
- **Pantalla de informes**: las pestañas Rendimientos y Ciudades vuelven a funcionar. El cálculo del
  efecto de las cartas de política, cargado en la misma pantalla, reemplazaba algunos de sus datos y
  las rompía. Las pestañas Políticas y Ciudades-estado ya no fallan en una fila con puntos de
  personajes ilustres, y ordenar una columna de la pestaña Ciudades ya no falla cuando no se abrió
  antes la pestaña Rendimientos.
- **Distritos con descuento**: el descuento mostrado es ahora el que aplica el juego. El juego solo
  actualiza el coste de un distrito en ciertos momentos (nueva tecnología o principio, nueva ciudad,
  distrito del mismo tipo colocado o terminado), así que la lista podía mostrar un descuento que el
  juego solo daba unos turnos más tarde.

## BetterGameExperience 0.9.0

Incluye todo lo de la versión preliminar 0.8.1. En multijugador, todos los jugadores humanos deben
tener la 0.9.0 para las correcciones **[todos los jugadores]**: una sala que mezcla 0.8.x y 0.9.0
las desactiva.

### Correcciones del multijugador [todos los jugadores]
- **Espectadores (Better Spectator Mod)**: ya no cuentan como civilizaciones mayores y salen de las
  listas de jugadores de los scripts, las victorias y la puntuación. BetterGameExperience incluye
  sus propias versiones de los scripts de Better Spectator Mod, hechas para ello. Contribución de
  banipouz.
- **Desincronización causada por un espectador**: cambiar el jugador que observa un espectador ya no
  desincroniza la partida (Better Spectator Mod lo escribía en los datos de la partida, solo en el
  equipo del espectador).
- **Espectadores en la cinta de líderes**: los jugadores ya no ven a los espectadores en la cinta de
  arriba a la derecha, donde aparecían como una IA, con un retrato « ? », su nombre y sus
  rendimientos.
- **Turno del espectador tras una pausa**: tras una pausa de MPH, el turno del espectador vuelve a
  terminar solo, en lugar de esperar a que lo pase a mano.

### Multijugador
- **Mensajes de red dañados**: un mensaje dañado, visto al desconectarse un jugador, podía congelar
  el juego hasta quedarse sin memoria unos minutos después. Los valores imposibles de un mensaje así
  se descartan ahora y la partida sigue; una línea en el chat lo dice a todos. Una configuración de
  partida o de jugador dañada ya no se aplica, en lugar de borrar la configuración. Protege a los
  jugadores que tienen BetterGameExperience. Contribución de yodha.
- **No First/Last moves con un anfitrión espectador**: el turno empieza ahora en cuanto los
  jugadores están listos, en lugar de retenerlos hasta el plazo de seguridad.

### Novedades
- **Progreso de investigación y principio**: en el panel superior izquierdo, junto al nombre de la
  tecnología y el principio en curso, **x/y** muestra la ciencia (la cultura) ya invertida y el
  coste total, seguido de **(+z)** cuando el juego guarda ciencia (cultura) sobrante de una
  tecnología (un principio) terminada con más que su coste. Un ajuste permite desactivarlo.
- **Disable Advanced UI indicators (BGE)**: nueva opción de partida, justo debajo de No First/Last
  moves, desactivada por defecto. Si el anfitrión la marca, ningún jugador ve el progreso de
  investigación y principio (**x/y (+z)**) ni la producción guardada de las ciudades, sean cuales
  sean sus ajustes. Se vuelve a leer en cada partida: en una partida sin la opción reaparecen, sin
  reiniciar Civ VI.
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
- **BGE Check**: en una sala multijugador, el anfitrión tiene un botón **BGE CHECK**, junto al MOD
  CHECK de MPH, que pide a cada jugador su versión de BetterGameExperience y escribe el resultado en
  el chat: misma versión, otra versión, instalado pero desactivado o sin respuesta.
- **Mensaje de llegada**: al abrir una sala con BetterGameExperience activado, el chat lo anuncia,
  con tu versión exacta.

### Correcciones de la interfaz
- **Panel de la ciudad tras retirar la producción**: el panel de abajo a la derecha se actualiza al
  retirar el único elemento de la cola, en lugar de seguir mostrándolo.
- **Clic derecho en la producción actual**: ahora también la retira desde la pestaña Producción, no
  solo con la cola abierta.
- **Sala de MPH**: fuera de un draft, la lista de jugadores sube al espacio vacío que MPH reserva
  para su draft, y desaparece el « . » que aparecía encima en una partida recargada.
- **Pantalla de diplomacia**: tras conocer a un jugador cuyo equipo faltaba en la lista de equipos
  del juego, los iconos de líderes fallaban cada turno, y abrir la pantalla de diplomacia podía
  ocultar toda la interfaz para siempre. Los iconos vuelven a funcionar y, si la pantalla de
  diplomacia no se abre, la interfaz vuelve.
- **Imhotep**: su efecto muestra ahora la cantidad a la velocidad de la partida (87, o 175 para una
  maravilla antigua o clásica, en velocidad En línea), en la pantalla de personajes ilustres y en la
  Civilopedia, en lugar de 175 en todos los casos.
- **Pausa de MPH**: el menú MULTIPLAYER HELPER y la votación de remap se muestran ahora por encima
  de la pantalla de pausa de MPH, con botones en los que se puede hacer clic.
- **Barra superior**: los servicios medios por ciudad se redondean al entero más cercano en lugar de
  hacia abajo.
- **Informar de un error (BGE)**: la partida guardada ahora se adjunta al informe; solo se
  enviaban los guardados automáticos.

### Ajustes
- Las novedades de interfaz (cancelar desde el estandarte, distritos rebajados, producción guardada,
  progreso de investigación y principio) y los scripts de Better Spectator Mod tienen cada uno un
  ajuste en la pestaña BetterGameExperience, activado por defecto.

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
