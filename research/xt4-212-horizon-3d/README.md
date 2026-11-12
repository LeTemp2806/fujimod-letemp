# X-T4 2.12: horizonte 3D persistente

Estado: investigación parcial e instrumentación de análisis implementada.
**No hay un parche funcional ni está identificada la causa del restablecimiento.**
No se generó una actualización, una tarjeta de arranque ni código para una cámara.

## Entrada y auditoría

Trabajo realizado sobre `main`, inicialmente sin cambios locales. No se crearon
ramas ni worktrees, no se hicieron commits ni push. Se leyeron las instrucciones
de desarrollo y los README de firmware, corpus, recovery, emulador y placa XP4.

Se localizó `/Users/sebastianrios/Downloads/FWUP0024.DAT` y se verificó:

| Propiedad | Resultado |
| --- | --- |
| Tamaño | 51 278 280 bytes |
| SHA-256 | `853d273b5603d3a2b93493f0ebd6a762b0b4b6bdc70317777775ae5f8ed93bc5` |
| Catálogo | `catalog/firmware/xt4/2.12/firmware.json`, coincidencia exacta |
| Cabecera exterior | tipo 6, palabras de versión `0x2`, `0x12` |

La palabra menor es 18 decimal (`0x12`), coherente con codificación BCD de 12;
el analizador conserva la palabra cruda y no la confunde con 12 decimal. La
identificación de versión se apoya en el hash completo y el catálogo, además
de la [página oficial de X-T4](https://www.fujifilm-x.com/en-us/support/download/firmware/cameras/x-t4/),
que identifica la versión 2.12. No se afirma una autenticación criptográfica.

El original se conserva intacto. El análisis se ejecutó sobre
`.tools/xt4-212-original.dat`, copia byte a byte. Los binarios extraídos y los
informes completos permanecen en `.tools/`, ignorado por Git.

Se reutilizan `fujimod_firmware::{scan, outer_header, sha256}` y Capstone, ya
dependencia de `fujimod-corpus`. No se añadió un descompresor ni dependencias.
`corpus` puede construir referencias y familias con límites de funciones
aportados por el llamador; este checkout no contiene un índice de funciones de
X-T4 que permita atribuir automáticamente las coincidencias a funciones.

## Evidencia reproducible

El ejemplo Rust `crates/corpus/examples/ui_references.rs` produce:

- Binarios separados de cada flujo method-4 validado por el decodificador.
- `references.generated.json`: coincidencias numéricas, materializaciones
  MOVW/MOVT, cargas literales, ramas directas y contexto desensamblado A32.
- `evidence.generated.json`: metadatos, cantidades, cadenas verificadas y rangos
  seleccionados con hash; su copia en este directorio procede del generador.

El mapa observado es:

| Offset en DAT | Bytes descomprimidos | Base declarada en cabecera |
| --- | ---: | --- |
| `0x00260274` | 7 420 408 | `0x01021000` |
| `0x005c0274` | 9 437 184 | `0x01e0f000` |
| `0x00ae0274` | 10 240 000 | Sin cabecera reconocida |
| `0x01a00274` | 6 446 348 | Sin cabecera reconocida |

La primera cabecera declara una extensión inicializada de 7 421 952 bytes,
mayor que los bytes realmente descomprimidos: no se inventó relleno ni BSS.
Las recetas existentes de `catalog/recovery/recipes.generated.json` corroboran
los hashes y bases de las secciones 5 y 6 y llaman a los otros bloques
`overlays` y `resources1`. El analizador no supone direcciones de ejecución para
esos dos bloques sin cabecera. Tampoco analiza todo el código crudo del DAT.

Todas las direcciones siguientes se derivan de las cabeceras de las dos
secciones principales; están verificadas como contenido estático de la copia,
**no como observaciones de ejecución en la cámara**. Los nombres descriptivos
de rutinas que siguen son etiquetas de investigación, no símbolos originales.

### Identificadores frente a comportamiento

En la primera sección, `0x0103af10` carga `0x544` (1348), lo compara con `r0`
y salta a `0x010411ec` cuando coincide. Ese caso materializa `0x0149bf34`, donde
se verificó la cadena `UI_SETP_3D_LEVEL`, y vuelve a una salida común. Es una
referencia al nombre del ajuste; **no es su activador**.

Otros MOVW que cargan `0x544` son mitades bajas de punteros o desplazamientos de
estructura. Por ejemplo, `0x01030930` va seguido de MOVT `0x142`: el valor
completo es `0x01420544`, no el identificador 1348. No se proponen parches sobre
estas coincidencias. Los cuatro IDs de escenas se inventariaron igualmente
como números, sin atribuirles handlers de captura.

`0x01065b78` inicia una rutina que devuelve nombres de valores: para `r0=0`
devuelve `UI_3D_LEVEL_ON` (`0x01462f38`), y para `r0=1` devuelve
`UI_3D_LEVEL_OFF` (`0x01462f48`). También tiene casos MAX, IGNORE_SET,
BACKUP_GET y PREV_GET. Esto describe un enum; no prueba cómo se almacena o
restaura el modo de visualización. `0x0105d204` materializa la cadena
`UI_EXE_REC_3D_LEVEL` (`0x0146b8e0`); tampoco es evidencia de un ejecutor.

### Estado interno encontrado

La cadena de diagnóstico `\t$INFO_REC boRec3DlevelStatus %s->%s\r\n` está en
`0x01564f40`. Una materialización MOVW/MOVT en `0x0113f624` la referencia.
El rango `0x0113f5c8..0x0113f644` contiene una rutina con prólogo y retorno:

1. Conserva `r0` en `r4`.
2. Comprueba condiciones de logging.
3. En `0x0113f5f8`, carga un puntero de la literal `0x0113f640`.
4. Esa literal contiene `0x01b7bd80`.
5. En `0x0113f5fc`, ejecuta `strb r4, [r3, #0xcbc]`.

Por tanto, esa ruta escribe el byte de `0x01b7ca3c`. La ruta de logging lee
el valor anterior, imprime el diagnóstico y vuelve a la misma escritura.
Es evidencia sólida de un **setter de estado informativo relacionado con 3D**;
no demuestra que este byte sea la preferencia `UI_SETP_3D_LEVEL`.

Se encontraron lecturas del desplazamiento `0xcbc` en `0x0114b638` y
`0x011577c8`. El segundo sitio transforma el resultado y lo guarda en
`[r4,#0x99]`. No se ha identificado el consumidor de ese campo ni el renderer
3D completo.

El barrido encuentra una rama directa candidata al setter en la sección 5
(`0x01149758`) y 189 en la sección 6. No son 190 funciones distintas: falta
comprobar alcance, límites, llamadas indirectas y rutas ejecutables.
Entre los sitios revisados:

| Sitio de llamada | Evidencia local |
| --- | --- |
| `0x01e3dbbc` | `mov r0,#0` inmediatamente antes del BL al setter |
| `0x01e9fa34` | `mov r0,#1` antes del BL; después llama `0x01fb21b0` |
| `0x01fd9250` | `mov r0,#1` antes del BL; también llama `0x01fb21b0` |
| `0x01fb21b0` | prepara ceros y llama `0x0112403c` con `r0=0xd9` |

No se ha identificado el significado del mensaje `0xd9`. No se confunde el
booleano del setter con el enum ON=0/OFF=1 del convertidor de nombres.
Suprimir globalmente escrituras de cero carece de justificación: existen
múltiples transiciones y podría impedir cambios manuales legítimos.

## Causa y solución

Las hipótesis A (reinicialización), B (pantalla temporal), C (restablecimiento
intencional por disparador) y D (preferencia no reaplicada) **siguen abiertas**.
No hay traza que relacione las escrituras encontradas con
`REC → disparador → captura → revisión → REC`. Tampoco se han identificado con
evidencia los handlers de disparo, el estado de revisión y el restaurador de REC.

La implementación entregada es la herramienta para obtener y revisar esa
evidencia, con rechazo estricto de entradas incorrectas y sin capacidad de
parcheo. No existe un mecanismo de persistencia implementado. Para poder
escribirlo falta identificar el escritor o transición que se ejecuta realmente
al disparar, establecer su relación con el ajuste y separar desactivación
voluntaria de limpieza temporal. Solo entonces corresponde una modificación
mínima con preimagen de instrucciones comprobada y pruebas de ambas rutas.

## Comparación con X-T5

El catálogo de X-T5 4.31 contiene `UI_SETP_3D_LEVEL=2037`, frente a 1348 en
X-T4. No se encontró un DAT local con la identidad de X-T5 4.31; no se hizo
comparación binaria entre ambas versiones.

El [manual oficial de X-T4](https://fujifilm-dsc.com/en/manual/x-t4/about_this_camera/display/index.html)
describe el acceso al 3D mediante un botón de función. El
[manual oficial de X-T5](https://fujifilm-dsc.com/en/manual/x-t5/about_this_camera/display/index.html)
describe un ajuste de pantalla OFF/2D/3D y un control asignable para alternar.
Esto aporta una diferencia de interfaz útil como referencia de diseño, pero
no prueba persistencia después de capturar ni representa necesariamente el
comportamiento exacto del firmware 4.31. No se trasladaron direcciones ni código.

## Reproducción local

Desde la raíz, con el Rust fijado en `mise.toml` y las dependencias instaladas:

```sh
mkdir -p .tools
cp -n /Users/sebastianrios/Downloads/FWUP0024.DAT .tools/xt4-212-original.dat
cargo run --locked -p fujimod-corpus --example ui_references -- \
  research/xt4-212-horizon-3d/analysis.json \
  .tools/xt4-212-original.dat .tools/xt4-horizon-review
cmp research/xt4-212-horizon-3d/evidence.generated.json \
  .tools/xt4-horizon-review/evidence.generated.json
```

El directorio de salida debe ser nuevo: se rechaza uno existente. Repetir en
otra salida produce el mismo informe. Para actualizar la evidencia se cambia
`analysis.json` o el generador, se vuelve a ejecutar y se copia íntegramente
`evidence.generated.json`. No editar ese archivo generado a mano. El informe
incluye el hash del archivo de especificación y los hashes de bloques y rangos.

El proceso verifica tamaño, hash completo y palabras de versión **antes de
descomprimir**. Rechaza rangos fuera de límites y cadenas de preimagen con cero
o varias coincidencias en los mapas declarados. Los resultados numéricos
ambiguos se conservan como candidatos, nunca autorizan modificaciones.

## Verificación realizada

```sh
cargo test --locked -p fujimod-firmware -p fujimod-corpus --lib --example ui_references
cargo clippy --locked -p fujimod-corpus --example ui_references -- -D warnings
pnpm run check:typescript-policy
```

Resultado: 12 pruebas de firmware, 26 de corpus y 10 de la herramienta pasan.
Clippy y la política TypeScript pasan. Hay dos avisos preexistentes de
dependencias de workspace no usadas en el manifiesto raíz. No se modificó
TypeScript; la compilación y comprobación de tipos afectadas son las de Rust.
Se utilizó Rust `nightly-2026-10-10`, instalado localmente en `.tools/` porque
no estaba disponible. La comprobación de política se ejecutó con Node 24 y
pnpm 11 disponibles en el entorno, que avisan de la diferencia con Node 26 y
pnpm 12 fijados por el proyecto; no se afirma haber validado aplicaciones web
ni bindings con ese entorno.

Las pruebas nuevas usan contenedores sintéticos para verificar identidad,
versión, instrucciones, límites, cadenas, reproducibilidad y negativa a
sobrescribir entradas/salidas. El análisis real verifica el DAT y descomprime
los cuatro bloques. **Ninguna de estas pruebas ejecuta la transición del
horizonte en el firmware.** Véase [SAFETY.md](SAFETY.md) para las pruebas
funcionales pendientes y el bloqueo de emulación/recuperación.
