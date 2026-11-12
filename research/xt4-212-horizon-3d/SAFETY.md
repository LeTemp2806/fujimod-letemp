# Estado de seguridad y validación

## Confirmado

- Identidad exacta del DAT X-T4 2.12 y extracción mediante el decodificador del
  proyecto, sobre una copia. El análisis no altera el original.
- Referencias estáticas y rangos desensamblados reproducibles; las direcciones
  declaradas no son una traza de la cámara.
- Hay recetas SD específicas para `xt4-212` en
  `catalog/recovery/recipes.generated.json`, con hashes, mapa de secciones y
  combinación de controles declarada. También hay una receta LED específica.
  Estas recetas son datos de construcción, no pruebas de arranque o recuperación.
- No se construyeron archivos de instalación, no se ejecutaron operaciones USB
  ni se accedió a hardware físico. No hubo push.

## Emulación: bloqueo concreto

`configurations/emulator/xt4-212/run.json` declara inicio `loader`,
`execution: null`, y entradas/perfiles XP4 de descubrimiento, parte de ellos con
nombres de piezas X-T3. Eso no acredita un modelo completo de X-T4.
`catalog/firmware/xt4/2.12/firmware.json` no contiene `boot_rom` ni
`installation`. Las comprobaciones de la placa XP4 validan componentes y
rechazos de operaciones no soportadas, no el arranque de este firmware.

El comando nativo de arranque documentado en `crates/emulator/README.md`
soporta X70. En `crates/emulator-boot/src/lib.rs`, `prepared` rechaza entradas
`loader` antes de ejecutar y también rechaza entradas XP4. Por tanto, ese
comando no puede ejecutar esta configuración de X-T4 tal como está.
Esta conclusión procede de inspección del código y la configuración: no se
afirma haber ejecutado una sesión XP4 ni medido un fallo de arranque.

El catálogo de progreso X-T4 marca REC, menús, captura, SD boot y todas las
validaciones en cámara como `not-run`. No se sustituyeron esas comprobaciones
por periféricos que respondan éxito ni por una máquina de estados ficticia.

Un ensayo aislado de la rutina de escritura es técnicamente más pequeño que
un arranque completo, pero solo demostraría que escribe el byte. Para un
ensayo útil de persistencia se necesitan los handlers reales, sus límites,
RAM inicial, tablas de eventos, overlays y servicios requeridos. Aún no se
dispone de esa preparación. El informe aporta puntos para construirla sin
confundir una simulación de la intención con una ejecución del firmware.

## Matriz de pruebas funcionales

Todas las filas siguen pendientes en firmware ejecutado, sin y con parche.

| Escenario | Evidencia que falta |
| --- | --- |
| Disparo único y vuelta a REC | traza de estado y renderer 3D antes/después |
| Disparos consecutivos | selección y actualización de inclinación sostenidas |
| Media pulsación | estado conservado y AF normal |
| LCD ↔ EVF | recreación de escena correcta, sin bloqueo |
| Cambio manual 3D → 2D | desactivación respetada tras nuevas capturas |
| Horizonte desactivado | ausencia de reactivación automática |
| Cambios de orientación | pitch/roll dinámicos y escenas H/V correctas |
| Playback → REC | restauración coherente con selección voluntaria |
| Apagado/encendido | ubicación y política de preferencia persistente |
| AF, exposición, imagen e IBIS | regresión sobre rutas reales de captura |

## Condiciones para continuar

Primero hay que relacionar `UI_SETP_3D_LEVEL`, el estado informativo encontrado,
los creadores/destructores de escenas y los eventos reales de captura. Un
watchpoint del byte candidato `0x01b7ca3c` en una ejecución correctamente
preparada sería una observación, no una escritura ni un mecanismo de solución.
Debe registrarse PC, argumentos, escena y motivo de transición antes/después
del disparo, comparándolos con la desactivación manual.

Después de demostrar la transición, el futuro parche debe fijar la identidad
exacta, comprobar preimágenes, rechazar ubicaciones ambiguas, limitar cambios a
la ruta demostrada y diferenciar aplicado/no aplicado. Debe validarse sin
hardware con la secuencia completa y sus regresiones. No se autoriza aplicar
las direcciones de esta investigación como un parche.

**El arranque SD y la recuperación segura de una X-T4 no están validados.**
Antes de considerar una prueba física deben existir evidencia específica de
arranque stock desde SD sin instalar nada, control de las lecturas/escrituras
que realiza el cargador y una vía de recuperación demostrada para esta cámara,
incluido el retorno al sistema original tras un fallo. Una luz LED, un hash,
un checksum o el éxito en otro modelo no satisfacen esas condiciones.
La autorización de esta misión excluye las pruebas físicas; cumplir estas
condiciones no concede autorización para realizarlas.
