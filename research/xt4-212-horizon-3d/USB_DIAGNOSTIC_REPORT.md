# X-T4 2.12: diagnóstico USB y horizonte 3D

Fecha: 12 de noviembre de 2026 (America/Lima). Investigación sobre `main`.
Estado: diagnóstico parcial, reproducción visual comunicada por el propietario
y nueva evidencia estática. **Causa interna todavía no identificada.**

## A. Resumen ejecutivo

- macOS enumeró `USB PTP Camera`, VID/PID `04cb:02e6`; el driver de
  libgphoto2 asigna esa pareja a X-T4. El propietario confirmó cámara conectada
  y encendida, modo **USB TETHER SHOOTING AUTO** y firmware **2.12**.
- El propietario reprodujo la desaparición del 3D tras una fotografía, activado
  con el botón físico **AEL**. La inclinación sin disparar, la media pulsación
  del disparador y el cambio LCD/EVF conservaron el 3D. Desactivarlo con AEL
  dejó el nivel 2D; después de disparar siguió desactivado el 3D.
- No se capturaron paquetes ni eventos PTP, RAM, registros o PCs ejecutados.
  No hay correlación USB–captura–REC demostrada. PLAY estaba inoperante según
  el propietario; atribuirlo al modo USB es una hipótesis, no una medición.
- Se encontró `po4yka/fuji-cli` (`fujicli`), pero sus afirmaciones disponibles
  de validación física son sobre X-T5 4.31. No se pudo verificar una matriz
  específica de X-T4; no se instaló ni ejecutó.
- Se detectó una limitación relevante de seguridad: la inicialización Fuji de
  libgphoto2 llama preparación de captura, que puede escribir `0xd38c` y
  `0xd207`. Una opción CLI de consulta no garantiza una sesión de solo lectura.
- Se amplió el análisis local: `0x0113fcec`, llamado junto al setter 3D en
  rutas ya conocidas, escribe un estado diagnosticado como `boFunctionStatus`.
  El helper del mensaje `0xd9` tiene numerosas referencias; no debe llamarse
  «evento de horizonte 3D» sin identificar sus consumidores.

El avance verificable es delimitar el desencadenante visual y una pareja de
estados internos para seguir investigando, además de establecer qué acceso
falta. No hay un parche listo ni autorización para probar uno en la cámara.

## B. Entorno técnico

| Elemento | Resultado y procedencia |
| --- | --- |
| Equipo | MacBook, `hw.model=MacBookAir10,1`, arquitectura `arm64` |
| Sistema | `sw_vers`: macOS 27.2.1, compilación 26B131 |
| Conexión | USB directo según el entorno de la misión; propietario confirma conectada y encendida |
| Modelo observado | Nombre USB genérico; identificación X-T4 por VID/PID y confirmación del propietario |
| Firmware instalado | 2.12, **declarado por el propietario**, no obtenido mediante GetDeviceInfo |
| Modo USB | USB TETHER SHOOTING AUTO, declarado por el propietario |
| Activación 3D | Botón físico AEL, ya asignado por el propietario; no se cambió su asignación |
| Revisión de imagen | Valor desconocido; no se modificó |
| Herramientas ejecutadas | `system_profiler`, `ioreg`, `sw_vers`, `uname`, `sysctl`, `tcpdump -D`, analizador local `ui_references`, verificaciones de hashes y política TypeScript |
| Disponibilidad CLI | `gphoto2`, `libusb-config`, `tshark` y `mise` no encontrados en PATH; esto no prueba ausencia de bibliotecas o instalaciones fuera de PATH |
| Dependencias nuevas | Ninguna instalada; sin permisos elevados |

La copia local del DAT tiene 51 278 280 bytes y SHA-256
`853d273b5603d3a2b93493f0ebd6a762b0b4b6bdc70317777775ae5f8ed93bc5`.
El analizador volvió a aceptar esa identidad y las palabras exteriores
`0x2/0x12`. Los cuatro bloques previamente extraídos se compararon por SHA-256
contra la evidencia existente: coincidieron. La coincidencia del archivo local
no autentica ni verifica los bytes instalados en la cámara.

## C. Herramientas encontradas

Las capacidades de esta tabla son documentales salvo las filas de enumeración
local. Compatibilidad de modelo, plataforma y firmware son cuestiones distintas.

| Herramienta | X-T4/macOS, información y eventos | Modo, lecturas y limitaciones |
| --- | --- | --- |
| [fuji-cli / fujicli](https://github.com/po4yka/fuji-cli) | Existe. Descubrimiento por esquemas y consulta PTP; X-T4 y esta versión de macOS no quedaron acreditadas mediante prueba física. No se verificó un monitor de eventos para este caso. | README distingue `device list` de `device info`, que abre sesión. La validación física anunciada es backup X-T5 4.31. No transferir ese contrato a X-T4. Documentos de soporte enlazados devolvieron 404; sin binarios publicados según README. Requiere herramientas de compilación/libusb, no instaladas aquí. |
| [libgphoto2](https://gphoto.sourceforge.io/proj/libgphoto2/support.php) y [gphoto2](https://github.com/gphoto/gphoto2) | X-T4 listada con captura, liveview y configuración. Código asigna `04cb:02e6`. CLI ofrece `--wait-event`; el resultado real depende de firmware y modo. Plataforma macOS contemplada, sin validación local de una instalación. | Tether AUTO ya seleccionado en esta cámara. **No ejecutar consultas o espera asumiendo ausencia de escrituras**: inicialización Fuji puede preparar captura. No instalado ni abierto aquí. |
| [libfuji](https://github.com/petabyt/libfuji) / CLI opcional `fuji` | Biblioteca de ingeniería inversa USB/Wi-Fi/BLE, tether y RAW; CMake contempla un CLI. No se encontró en las fuentes consultadas validación física específica X-T4 2.12/macOS 27.2.1. | `fujiusb_setup` abre sesión, obtiene DeviceInfo y consulta modo USB. Otras rutas cargan objetos y escriben propiedades RAW. Su nota USB cita otros PID; no acredita X-T4. No usar el ejecutable completo como sonda inocua. |
| [libusb](https://libusb.info/) | Soporta macOS como transporte. Puede enumerar descriptores y efectuar transferencias bulk/control/interrupt; no interpreta por sí solo horizonte, REC ni PTP. | Enumerar no requiere cambiar modo; reclamar/configurar interfaces puede afectar su uso por otras aplicaciones. No es un sniffer pasivo del tráfico ajeno. No se instaló ni se enviaron transferencias con él. |
| `ioreg` / `system_profiler` | Lectura del inventario del sistema. `IOUSBHostDevice` sí mostró la pareja Fuji; otras consultas no dieron una vista completa. | No modifica el modo USB. No muestra llamadas internas ni eventos de captura; la ausencia en una vista no prueba desconexión física. |
| [Wireshark: captura USB](https://wiki.wireshark.org/CaptureSetup/USB), `tcpdump` | Documentación describe XHC20 para macOS y restricciones en versiones posteriores. `tcpdump -D` local no enumeró XHC/USB. | No se inició captura, no se habilitaron interfaces ni se tocó SIP. La ruta histórica documentada no demuestra que funcione en este Mac/macOS. Un analizador USB externo es una alternativa futura, no equipo disponible validado. |
| FujiMod `fujimod-usb`, App80 y workflows | Descubrimiento nusb, transporte ff80, consultas, consola/logs y trazas de **sus propias** operaciones. Nada de ello acredita acceso de depuración en USB PTP normal. | `Connection::open` puede configurar interfaz; `begin_session` usa protocolo de servicio. Workflows pueden escribir RAM/configuración, habilitar debug o ejecutar código. Excluidos en esta misión, incluso si alguna API se llama «read». |
| FujiMod `raw-conversion` | Implementación PTP bulk reutilizable y parser DeviceInfo. No es monitor del horizonte. | El flujo RAW sube RAF y configura conversión: no ejecutado. Su parser actual descarta OperationsSupported y EventsSupported, por lo que falta conservarlos para un inventario diagnóstico completo. |
| [Fujihack](https://github.com/fujihack/fujihack) | Fuente de investigación sobre firmware y protocolos. No se verificó una herramienta de observación 3D para X-T4 2.12. | Referencia para análisis fuera del dispositivo; no se ejecutaron procedimientos de servicio, exploits o instalación. |

Fuentes concretas: [README fujicli](https://raw.githubusercontent.com/po4yka/fuji-cli/main/README.md),
[CLI opcional libfuji](https://raw.githubusercontent.com/petabyt/libfuji/master/CMakeLists.txt),
[notas USB libfuji](https://raw.githubusercontent.com/petabyt/libfuji/master/docs/usb.md).
La API pública de GitHub de `po4yka/fuji-cli` también devolvió 404 durante esta
sesión pese a que el README y la página indexada fueron recuperables. Se conserva
esa incertidumbre; no se afirma haber auditado una revisión completa del proyecto.

## D. Diagnóstico USB

### Lecturas realizadas

Se ejecutaron consultas al inventario de macOS, filtrando los campos publicados
por lista permitida y excluyendo números de serie, UUIDs, rutas de ubicación y
otros identificadores privados. No se guardó un volcado USB crudo en el reporte.

| Consulta | Resultado observado |
| --- | --- |
| `system_profiler SPUSBDataType -json` | Lista `SPUSBDataType` vacía; no suficiente para descartar la cámara |
| `ioreg -p IOUSB -a` | Cero nodos con idVendor en esa vista |
| `ioreg -r -c IOUSBHostDevice -a` | Una cámara con los campos de la tabla siguiente en varias consultas |
| `ioreg -r -c IOUSBHostInterface -a` | Salida no interpretable como plist; no se obtuvieron descriptores de interfaz |
| `ioreg -p IOService -r -c IOUSBHostInterface -l` | Sin líneas de descriptor coincidentes |
| `ioreg -p IOService -a` filtrado por VID | Sin coincidencias; no se determinó la razón de la discrepancia entre vistas |
| `tcpdump -D` | 19 interfaces enumeradas, ninguna XHC/USB; ninguna captura iniciada |

Una consulta posterior a IOUSBHostDevice también devolvió salida vacía/no válida
para el parser. La última repetición, a las **09:43:33 America/Lima**, volvió a
mostrar un nodo (comando con salida exitosa). No atribuir estas diferencias a
apagado, reinicio, disparo ni desconexión sin evidencia adicional.

| Campo USB | Valor observado |
| --- | --- |
| Product Name | `USB PTP Camera` |
| idVendor | `1227 = 0x04cb` |
| idProduct | `742 = 0x02e6` |
| bcdDevice | `530 = 0x0212` |
| bNumConfigurations | `1` |
| bDeviceClass / SubClass / Protocol | `0 / 0 / 0` |

Clase de dispositivo cero no permite inferir una clase de interfaz concreta.
El nombre y PID son compatibles con PTP/X-T4; no sustituyen leer los descriptores
de interfaz. **No se midieron** número de interfaz, clase 6/1/1, endpoints bulk o
interrupt, tamaños de paquete, velocidad, controlador propietario ni acceso ff80.
`bcdDevice=0x0212` tampoco demuestra por sí solo firmware 2.12.

### Operaciones PTP: conocidas frente a comprobadas

**Ninguna operación PTP fue enviada en esta sesión.** Los únicos comandos de
lectura comprobados son los del inventario del host indicados arriba.

Para una futura sonda con transporte auditado, las operaciones estándar
`GetDeviceInfo (0x1001)`, `GetDevicePropDesc (0x1014)` y
`GetDevicePropValue (0x1015)` son consultas documentadas. GetDeviceInfo permite
inventariar versión, operaciones, eventos y propiedades anunciados; las otras
dos deben limitarse a propiedades anunciadas y de semántica conocida. Que un
evento figure como soportado no demuestra su emisión en un disparo manual.
Abrir/cerrar sesión cambia el estado de protocolo; configurar/reclamar una
interfaz también puede influir en el comportamiento observado. Deben revisarse
esas etapas antes de afirmar que una herramienta completa es de solo lectura.

Hallazgo de código: `library.c` inicializa Fuji mediante `camera_prepare_capture`;
`config.c` prepara modo PC con `SetDevicePropValue(0xd38c,1)` y prioridad USB
con `SetDevicePropValue(0xd207,2)` cuando la propiedad está disponible. Por ello
`gphoto2 --summary`, `--list-config` o `--wait-event` no quedan autorizados aquí
como consultas puras por su nombre. Véanse
[inicialización del driver](https://github.com/gphoto/libgphoto2/blob/master/camlibs/ptp2/library.c)
y [preparación Fuji](https://github.com/gphoto/libgphoto2/blob/master/camlibs/ptp2/config.c).
Los números de línea del upstream cambian; buscar los símbolos indicados.

`ObjectAdded (0x4002)` podría indicar un objeto disponible y
`CaptureComplete (0x400d)` finalización de captura si realmente se emiten.
Ninguno significa automáticamente revisión de imagen terminada o retorno de la
UI a REC. No se encontró una consulta documentada, validada para esta X-T4, que
lea la selección o visibilidad del horizonte 3D. **No se concluye que no exista**:
faltan inventario vivo y contrato de sus propiedades. El USB PTP estándar no
proporciona por sí mismo watchpoints ni trazas de llamadas de firmware.

## E. Investigación del horizonte 3D

### Evidencia reutilizada

Se leyeron README, SAFETY, especificación y evidencia previas, analizador,
módulo de referencias y configuración del emulador, sin modificar esos archivos.
Las direcciones siguientes pertenecen únicamente al DAT de identidad verificada:

| Referencia | Lo que demuestra y lo que no |
| --- | --- |
| `UI_SETP_3D_LEVEL=1348 (0x544)` | ID del ajuste; no es un opcode PTP |
| `0x01065b78`, cadenas `UI_3D_LEVEL_ON/OFF` | Conversión de nombres de enum ON=0/OFF=1; no atribuir esa codificación al byte informativo |
| `UI_EXE_REC_3D_LEVEL` | Cadena identificada; no ejecutor demostrado |
| `0x0113f5c8`, escritura `0x0113f5fc` | Setter informativo; literal `0x01b7bd80` más `0xcbc` produce `0x01b7ca3c` |
| `0x0114b638` | Lectura y comparación con 1; consumidor completo todavía desconocido |
| `0x011577c8..0x011577d4` | Lee estado y guarda en `[r4,#0x99]` el resultado de `estado != 1`, no una copia directa del byte |
| `0x01e3dbbc` | Llamada al setter con `r0=0`; sin vínculo demostrado con disparo |
| `0x01e9fa34`, `0x01fd9250` | Llamadas con `r0=1`, seguidas de `0x01fb21b0` |
| `0x01fb21b0` | Prepara ceros y llama `0x0112403c` con `r0=0xd9`; semántica del mensaje sin resolver |

### Ampliación estática de esta sesión

Se utilizó el generador existente con una especificación adicional
[usb-analysis.json](usb-analysis.json), sin editar archivos generados a mano.
Salida compacta íntegra: [usb-evidence.generated.json](usb-evidence.generated.json).

1. `0x0113fcec` tiene prólogo, conserva `r0` en `r4`, y en
   `0x0113fd30` ejecuta `strb r4,[r3,#0x82a]`.
2. La carga PC-relative de `0x0113fd2c` usa la literal `0x0113fd74`, cuyo
   contenido es `0x01b7dd80`. El destino estático es **`0x01b7e5aa`**.
   Es una estructura base distinta de la del estado 3D.
3. La rama de logging materializa `0x0156661c`: cadena verificada con terminador
   NUL `\t$INFO_REC boFunctionStatus  %s->%s\r\n`. Etiqueta de investigación:
   **setter informativo de boFunctionStatus**, no símbolo original.
4. Las ventanas previas muestran que las rutas `0x01e9fa28` y `0x01fd9244`
   lo llaman con 1 antes de poner el estado 3D a 1. La ruta de limpieza
   `0x01e3dbb4` lo llama antes de poner 3D a 0; falta seguir todos sus predecesores.
5. El barrido encuentra 1 + 56 ramas directas candidatas a `0x0113fcec` en
   las dos secciones mapeadas y **188** al helper `0x01fb21b0` en la segunda.
   No son funciones distintas ni rutas ejecutadas verificadas. Esta amplitud
   aconseja investigar si el mensaje es compartido con otras funciones de UI.

La nueva ventana cubre `0x0113fcec..0x0113fd78` (fin exclusivo, incluye literales).
Su SHA-256 es
`449133d977acafd2aa8aa619c43f8741209261f0c26da3402066194d31eea831`.
El valor debe contrastarse con el archivo generado al reproducir; no es una
preimagen para autorizar cambios en el dispositivo.

Esto añade una relación concreta entre dos estados informativos. **No demuestra**
que `boFunctionStatus` represente una pantalla temporal, que `0xd9` restaure REC
o que alguno de los bytes sea la preferencia del usuario.

### Reproducción fuera de la cámara

Con la copia DAT y las dependencias ya disponibles, en una salida nueva:

```sh
cargo run --locked -p fujimod-corpus --example ui_references -- \
  research/xt4-212-horizon-3d/usb-analysis.json \
  .tools/xt4-212-original.dat .tools/xt4-usb-reproduction
cmp research/xt4-212-horizon-3d/usb-evidence.generated.json \
  .tools/xt4-usb-reproduction/evidence.generated.json
```

Esta sesión invocó el ejecutable `target/debug/examples/ui_references` ya
disponible, con esos tipos de entrada, sin compilar ni instalar dependencias.
La extracción volvió a verificar DAT, versión y cadena. La inferencia del destino
del setter se contrastó también leyendo la palabra literal del bloque cuyo hash
coincide. Los binarios y referencias completas están en `.tools/`, ignorado por
Git. No se redistribuyen bytes del firmware en JSON.

## F. Experimentos realizados

**Procedencia:** las observaciones visuales son mensajes del propietario durante
esta sesión, no vídeo revisado por el agente ni telemetría USB. Se conserva el
alcance literal de cada respuesta. Las horas de cada acción manual no se
registraron; no existe sincronización con una captura de tráfico.

| ID | Pasos realizados/comunicados | Resultado observado | Límite |
| --- | --- | --- | --- |
| M0 | AEL activa 3D; inclinar sin disparar | «sigue ok el horizonte3D» | Sin medidas de pitch/roll; confirma respuesta visual comunicada |
| M1 | Activar 3D; tomar una foto; observar después | «tomé una foto y desapareció» | No permite situar el instante dentro de obturación, revisión o retorno a REC; no se confirmó si quedó 2D tras esa captura |
| M2 | Activar 3D; media pulsación **del disparador**, soltar sin fotografía | «el 3D permanece» | Se aclaró que media pulsación no era del botón AEL; no se midió AF/exposición |
| M3 | Tres fotos separadas, esperando vista de disparo, sin reactivar AEL | Propietario confirma tres fotos y desaparición tras la primera | No especificó por separado el estado después de la segunda y tercera; reaparición espontánea aún no evaluada de forma explícita |
| M4 | Intentar PLAY y retorno a REC sin disparar | PLAY desactivado/inoperante según propietario | No se ejecutó el ciclo; posible influencia de tether, sin demostrar causa |
| M5 | Con 3D activo, acercar ojo y observar EVF | Presente en pantalla principal y también en EVF | No vídeo; no se confirmó por separado todo un ciclo repetido LCD→EVF→LCD |
| M6 | Desactivar 3D con AEL; disparar | Quedó línea 2D; 3D permaneció desactivado después de la foto | Observación del comportamiento stock; no valida un futuro parche |
| M7 | Revisión activada frente a desactivada | **No realizado** | Requiere autorización específica para cambiar configuración y restaurar valor original |

No se instaló firmware, no se escribieron RAM/Flash o archivos de cámara,
no se ejecutaron payloads, comandos ff80, reinicios ni transiciones de servicio.
Las fotografías manuales fueron los ensayos solicitados por la misión.

### Protocolo pendiente para completar la matriz

Antes de otra serie registrar modo USB, revisión actual, pantalla y orientación,
sin modificar configuraciones. Una grabación externa del LCD, encuadrada sin
datos privados, permitiría identificar el último frame con 3D, la revisión y el
primer frame de REC; anunciar ID y acción en voz alta serviría de referencia.
No suponer que un stream de liveview USB incluye los overlays del LCD/EVF.

- Repetir M1 al menos tres veces reactivando con AEL antes de cada foto;
  registrar 3D/2D/ausente y si responde a inclinación al volver.
- Completar M3 con una anotación separada después de cada foto.
- Completar LCD→EVF→LCD y repetir disparo en cada pantalla, manteniendo los
  ajustes actuales. Incluir orientación moderada para evitar la limitación
  documentada al apuntar muy arriba/abajo.
- Repetir M6 para comprobar que el estado voluntario 2D nunca reactiva 3D.
- PLAY→REC queda pendiente mientras PLAY no funcione. Compararlo en otro modo
  de conexión requiere autorización para ese cambio; no forzarlo por USB.
- Comparar revisión ON/OFF solo tras autorización expresa, registrando y
  restaurando el valor previo. Mantener constantes pantalla y modo USB.

Para cada ensayo registrar: ID, hora con zona, selección inicial, acción,
pantalla, resultado final, dinamismo del nivel, revisión observada y fuente
(operador/vídeo/evento decodificado). Si luego hay PTP, guardar código, parámetros
y hora del evento por separado de la interpretación visual; omitir nombres de
fotos, handles e identificadores privados del reporte público.

## G. Hipótesis y causa probable

| Hipótesis | Evaluación actual | Evidencia discriminante que falta |
| --- | --- | --- |
| A: reinicialización del estado | Compatible con desaparición poscaptura; setter y ruta con cero existen | Observar qué escritor se ejecuta y el valor real antes/después |
| B: destrucción/recreación del gráfico | Compatible; media pulsación y cambio de pantalla conservaron 3D, por lo que no toda transición visual lo elimina | Ciclo de vida del renderer durante captura y retorno a REC |
| C: desactivación intencional por evento del disparador | Media pulsación sola no bastó; sigue posible un evento de pulsación completa/captura | Handler de pulsación completa y su transición real; no confundir esto con el botón AEL |
| D: selección conservada, no reaplicada | Compatible, pero no está demostrada una preferencia persistente 3D | Relación entre ajuste 1348, estado informativo y reconstrucción de escena |
| E: vista de función transitoria | Pareja con `boFunctionStatus` y activación mediante AEL justifican estudiarla | Despacho de la función, salida del modo y consumidores del mensaje `0xd9` |
| F: influencia de tether/revisión/orientación | USB AUTO puede alterar el contexto; PLAY inoperante y revisión desconocida impiden excluirlo | Ensayos comparables autorizados; controles de inclinación y revisión |

No hay base para escoger una causa interna probable única. La conclusión más
estrecha es: **en el contexto tether observado, tomar una foto elimina el 3D
activado mediante AEL; media pulsación sola, inclinación y acceso al EVF no lo
eliminaron**. Esto centra la siguiente investigación en pulsación completa,
captura y salida de una función de UI, sin demostrar aún cuál de ellas manda.
El manual admite pérdida de horizonte con inclinaciones extremas, pero no
describe en el pasaje consultado la persistencia poscaptura del 3D.

## H. Propuesta técnica para el futuro parche

Seguir conjuntamente el ajuste `UI_SETP_3D_LEVEL`, los setters
`0x0113f5c8` y `0x0113fcec`, el helper `0x01fb21b0` y los consumidores de
`[r4,#0x99]`. Priorizar los predecesores de la ruta `0x01e3dbbc` y distinguirlos
de la desactivación voluntaria mediante AEL. Resolver el despacho de `0xd9`
antes de atribuirle significado de captura o REC.

Si se demuestra que una salida temporal limpia el estado y no lo restaura,
la modificación candidata sería reaplicar la **selección voluntaria vigente**
en el retorno concreto a REC, o corregir la reconstrucción del elemento gráfico
si el estado ya es correcto. Son alternativas condicionadas por la evidencia,
no parches propuestos sobre instrucciones específicas. Una desactivación manual
debe invalidar cualquier intención pendiente de restauración. No bloquear
globalmente escrituras de cero ni convertir el estado a uno permanente.

Antes de implementar faltan límites y alcance de handlers, semántica de los
estados, ciclo del renderer, una traza real del flujo y pruebas de regresión.
El emulador X-T4 declara `start=loader` y `execution=null`; el arranque nativo
actual rechaza loader/XP4. No se dispone de ejecución de la secuencia stock
que permita sustituir estos ensayos por una prueba de emulación validada.
Un watchpoint de lectura/observación podría resolver el escritor **solo en una
ejecución correctamente preparada y autorizada**, nunca activando debug físico
en esta misión. No se ha demostrado una vía de lectura de logs stock por PTP.

## I. Plan de continuación

1. Completar evidencia visual de M1/M3, incluyendo si al desaparecer 3D queda
   2D y si una pulsación de AEL vuelve a activarlo. Sin nuevas dependencias.
2. Continuar análisis fuera del dispositivo de `boFunctionStatus`, las llamadas
   de limpieza, dispatcher `0xd9` y renderer. Mantener identidad del DAT fijada.
3. Resolver la enumeración de **interfaces** con una herramienta de descriptores
   auditada; el inventario del host actual es insuficiente. No reclamar ni
   configurar automáticamente una interfaz ni desalojar aplicaciones del sistema.
4. Preparar y revisar una sonda PTP mínima que conserve listas de operaciones,
   eventos y propiedades, excluya números de serie y no llame inicializadores
   de captura, setters, reset ni comandos de servicio. Someter cualquier apertura
   o cambio de comportamiento no probado de solo lectura a autorización previa.
5. Solo con ese transporte justificado, correlacionar eventos realmente emitidos
   con vídeo y disparos manuales. Un evento de archivo no prueba retorno a REC.
6. Pedir autorización concreta si hace falta instalar dependencias, cambiar USB
   o comparar revisión ON/OFF. No se necesita hacerlo para conservar este avance.
7. Si PTP no aporta estado interno, continuar estáticamente o preparar emulación
   real fuera del dispositivo. La carencia concreta es una traza de escritor,
   argumentos, eventos y escena del flujo stock; ninguna herramienta validada
   disponible en esta sesión la proporciona.

### Validación local de la entrega

- `pnpm run check:typescript-policy`: pasó; Node 24.22.1/pnpm 11.25.0 avisaron
  diferencia con Node >=26.12.0/pnpm 12.18.0 requeridos por el proyecto.
- Generación adicional: identidad, versión y cadena verificadas por la
  herramienta existente. Archivo generado copiado íntegro, sin cambios manuales.
- Segunda generación en una salida nueva: `cmp` confirmó evidencia idéntica
  byte por byte. Se verificaron hashes de bloques y ventana, enlaces locales y
  ausencia de identificadores privados en el reporte; `git diff --check` pasó
  para cambios rastreados (los tres archivos nuevos aún no están en el índice).
- No se cambió código mantenido, configuración TypeScript ni firmware; no se
  requieren typechecks o tests funcionales nuevos por esta entrega documental.
- No se hicieron commits, push, ramas, worktrees ni ejecuciones de workflows.

## J. Referencias

### Repositorio local

- [Instrucciones del proyecto](../../AGENTS.md).
- [Investigación previa](README.md), [seguridad y emulación](SAFETY.md).
- [Especificación previa](analysis.json), [evidencia previa](evidence.generated.json):
  setter alrededor de línea 300, lector B alrededor de 790, rutas cruzadas
  alrededor de 890 y notificador alrededor de 1100.
- [Generador](../../crates/corpus/examples/ui_references.rs): identidad y versión
  líneas 78–96; extracción y referencias 133–170; escritura de evidencia 191–220.
- [Barrido A32](../../crates/corpus/examples/ui_references/analysis.rs): referencias
  líneas 31–123; candidatos, no límites de función ni prueba de ejecución.
- [Entrada adicional](usb-analysis.json), [evidencia adicional](usb-evidence.generated.json).
- [Transporte USB](../../crates/usb/src/connection.rs): filtro VID/PID líneas
  92–95; [README](../../crates/usb/README.md): configuración al abrir líneas 44–48.
- [Trazas propias ff80](../../crates/usb/src/trace.rs): líneas 1–2.
- [Workflows con efectos](../../crates/workflows/README.md): líneas 8–20 y 54–80.
- [PTP local](../../crates/raw-conversion/src/ptp.rs): opcodes líneas 9–16,
  OpenSession 84–100; parser descarta listas de operaciones/eventos 285–303.
- [Conversión RAW](../../crates/raw-conversion/src/run.rs): propiedades/operaciones
  de escritura líneas 13–17; [protocolo](../../crates/raw-conversion/README.md).
- [Configuración X-T4](../../configurations/emulator/xt4-212/run.json): líneas 8–11;
  [rechazos del arranque](../../crates/emulator-boot/src/lib.rs): líneas 298–317.

### Documentación y código externos consultados

- [Manual X-T4: pantallas y horizonte virtual](https://fujifilm-dsc.com/en/manual/x-t4/about_this_camera/display/index.html).
- [Manual X-T4: modos de conexión](https://fujifilm-dsc.com/en/manual/x-t4/menu_setup/connection_setting/index.html).
- [Lista oficial libgphoto2](https://gphoto.sourceforge.io/proj/libgphoto2/support.php).
- [Driver PTP libgphoto2](https://github.com/gphoto/libgphoto2/blob/master/camlibs/ptp2/library.c):
  buscar `Fuji:Fujifilm X-T4`, `case PTP_VENDOR_FUJI`, `camera_prepare_capture`.
- [Configuración libgphoto2](https://github.com/gphoto/libgphoto2/blob/master/camlibs/ptp2/config.c):
  `camera_prepare_capture`, rama Fuji, escrituras `0xd38c` y `PriorityMode`.
- [Definiciones PTP](https://github.com/gphoto/libgphoto2/blob/master/camlibs/ptp2/ptp.h):
  `PTP_OC_GetDeviceInfo`, `PTP_OC_GetDevicePropDesc`, `PTP_OC_GetDevicePropValue`,
  `PTP_EC_ObjectAdded`, `PTP_EC_CaptureComplete`.
- [CLI gphoto2](https://github.com/gphoto/gphoto2/blob/master/gphoto2/main.c):
  opciones `wait-event` y `wait-event-and-download`, de efectos diferentes.
- [fuji-cli](https://github.com/po4yka/fuji-cli), [README recuperado](https://raw.githubusercontent.com/po4yka/fuji-cli/main/README.md).
- [libfuji](https://github.com/petabyt/libfuji),
  [implementación USB](https://github.com/petabyt/libfuji/blob/129200dfde67f19da58cec55cf24c0386374f707/lib/fuji_usb.c):
  `fujiusb_setup`, consultas y rutas de conversión.
- [libusb](https://libusb.info/), [captura USB Wireshark](https://wiki.wireshark.org/CaptureSetup/USB),
  [Fujihack](https://github.com/fujihack/fujihack).

Las fuentes externas describen capacidades y riesgos; ninguna constituye una
traza interna de esta cámara. No se publican identificadores privados.
