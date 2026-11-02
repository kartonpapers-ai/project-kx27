# Project KX27: paleta de trapping

Estado: extremos locales de curvas KX27.9, 2026-11-02, sobre el flujo de incorporación de franjas locales revisadas. Se conservan la captura por tintas y el trapping de sólidos, spots, degradados lineales e imágenes opacas. No cambia la calificación alfa del editor.

## Ventana de desarrollo guiada

Abre **Window / Ventana → Trapping · Desarrollo**, o pulsa **Abrir ventana de desarrollo** al principio de la paleta Trapping. Es una ventana flotante amplia, redimensionable y no modal: puedes seguir trabajando en el documento. Su apertura se guarda en preferencias. Mientras está abierta, la paleta habitual muestra un acceso a esta ventana; al cerrarla vuelven sus controles anteriores.

El recorrido tiene seis pasos numerados:

1. **Detectar:** calcula el mapa de candidatos con los ajustes de resolución y contraste. No necesitas seleccionar todo el arte.
2. **Zona y par:** elige una zona y después un par de originales. Si hay varios pares, la elección es explícita; el mapa también permite elegir zonas. Los originales se resaltan en el documento.
3. **Verificar curvas:** consulta las curvas del par elegido dentro de esa zona. Las muestras visibles permiten avanzar; no son una prueba continua de contacto ni una decisión automática de trapping.
4. **Previsualizar:** revisa ancho, origen, receptor, sentido y franja temporal. Cambiar ancho o sentido invalida la franja y exige previsualizar otra vez.
5. **Incorporar:** confirma la franja revisada. Este es el único paso del recorrido que modifica el dibujo; mantiene las mismas comprobaciones de tintas, vigencia y deshacer de la incorporación local.
6. **Diagnóstico:** exporta el JSON, también después de un error. Tras incorporar muestra el resultado vigente y recuerda que Edición → Deshacer revierte esa incorporación en un paso.

El botón principal permanece abajo junto a **Anterior**; el contenido y los avisos tienen desplazamiento propio. En ventanas estrechas, **Paso** sustituye la lista lateral. Zona, par e identidades de los originales acompañan la revisión: no se deducen de una selección posterior distinta. Solo se habilitan los pasos con sus requisitos disponibles.

La ventana comparte capturas y consultas con la paleta. Cambiar el documento, arte, mesa o ajustes de color invalida los resultados correspondientes. Los controles no calculan geometría durante los fotogramas normales. El comando de interfaz `trapping.developmentWindow` acepta `{open?:bool}`; sin parámetro abre la ventana. `settings.development` añade al diagnóstico el paso, la zona/par vigentes y el estado de incorporación, sin adjuntar imágenes ni geometría completa del archivo.

La prueba de mezcla de coberturas de tintas compartidas queda pendiente. Esta entrega organiza la interfaz; no modifica el motor ni permite reducir la cobertura de una tinta compartida. **Captura por tintas** y **Aplicar a la selección** siguen en la paleta habitual, al cerrar esta ventana; no forman parte del recorrido local nuevo.

Validación: 7 pruebas del recorrido nuevo y 18 de la paleta existente; capturas nativas egui en temas claro/oscuro y tamaños 1280×800, 1024×700 y 700×560; compilación WASM. La ejecución completa de `cargo xtask ci` se detiene en avisos Clippy previos en `render/src/fx.rs` y `pdf/src/import_scan.rs`; no se atribuye un pase global de CI a esta entrega.

## Extremos de franjas en cruces de curvas

KX27.9, 2026-11-02. La caja de una zona del mapa es una aproximación rasterizada: antes podía cortar el contacto antes del cruce real. La previsualización local recibe ahora `zoneDpi` desde la captura y amplía la búsqueda por el mayor de dos márgenes: ancho nominal más 0,1 puntos, o diagonal de un píxel del detector más 0,05 mm. Sin `zoneDpi`, una consulta con región explícita conserva su margen anterior; no se convierte en trapping del objeto completo.

Los segmentos Bézier originales se separan en sus cruces con los contornos y clips locales y con el límite de búsqueda. La intersección es numérica y acotada, con tolerancia de 0,0001 puntos; las celdas terminales comprueban su interior para no rechazar el extremo por un sondeo al otro lado del cruce. No se unen puntos del mapa rasterizado. Se mantienen el ancho nominal, extremos rectos y el recorte completo al receptor visible, incluidos objetos que lo ocultan y agujeros. Hasta 120.000 visitas de intersección y 8.192 cortes por segmento; superar el presupuesto devuelve un error sin incorporar geometría parcial. No es una demostración continua de contacto ni una prueba RIP; tangencias y coincidencias degeneradas siguen requiriendo revisión.

`trapping.previewLocal` acepta `zoneDpi?:number` entre 36 y 600 ppp. El informe añade `zoneDpi`, `mapHaloPt`, `endpointMethod`, `endpointTolerancePt` y `terminalChecksPerInterval`; `checksPerInterval` es el máximo de sondeos. La incorporación usa la franja revisada, sin cambiar las reglas de tintas compartidas ni las omisiones manuales.

Para repetir una prueba antigua, deshaz o elimina primero su franja incorporada, detecta nuevamente y recorre los pasos 2 a 5. Los traps existentes no se recalculan al abrir la nueva versión. Ajusta el ancho en **Paso 4 · Previsualizar**: el ancho del Apply antiguo es un control separado; el flujo local admite de 0,01 a 0,50 mm.

Validación: ocho pruebas de previsualización, incluidas dos elipses sintéticas a 36/72/150 ppp con anchos 0,01/0,1/0,5 mm, ambos extremos a menos de 0,003 puntos del cruce independiente, ancho geométrico, papel, tercer objeto y región explícita. También pasan ocho pruebas de curvas y siete de incorporación (vigencia, sobreimpresión, PDF, deshacer y rechazo de reducciones de tintas compartidas). En la reconstrucción local del caso privado del operador, el extremo antes corto (unos 0,154 mm) queda a menos de 0,00002 mm del cruce numérico independiente; la incorporación y exportación PDF también terminan. El caso se utiliza solo localmente; no se incorpora al repositorio. También pasan las 28 pruebas de la paleta y el recorrido, formato, assets, brands y layers. WASM compila. La CI global continúa detenida por avisos anteriores de Clippy en `render/src/fx.rs` y `pdf/src/import_scan.rs`; no se declara un pase global.

## Omitir trapping por decisión del operador

KX27.8, 2026-11-02. En la ventana de desarrollo, elige una **Zona** y un **Par** concreto. En los pasos 2 a 5 aparece **Omitir trapping**, encima de la acción principal. No crea ni elimina objetos, no modifica tintas y no añade un paso a Deshacer. Es una decisión manual: no significa que el programa haya confirmado que ese encuentro no necesita trapping.

El par queda marcado **Omitido · sin franja** y en su lista figura **Omitido**. La franja temporal y la verificación de ese par se descartan y el recorrido vuelve a **Zona y par**; la verificación y la incorporación quedan bloqueadas en esta ventana mientras esté omitido. **Volver a evaluar** retira la omisión y permite verificarlo nuevamente.

**Revisar otro par** selecciona el siguiente par atribuible sin omitir, pasando a otra zona cuando corresponde. Si no hay otro, abre **Diagnóstico**, que enumera las omisiones y ofrece regresar a Zona y par. Los objetos se resaltan a través de `select.set`, sin modificar el arte.

**Alcance inicial: análisis actual, no archivo nativo.** Las decisiones viven en la sesión del panel y el JSON; no se guardan como reglas permanentes en el documento ni bloquean los comandos directos del motor o el Apply antiguo. Cambiar el arte (incluida la incorporación de otra franja), documento, mesa, CMS o ajustes de detección, o detectar nuevamente, descarta estas omisiones. Exporta el diagnóstico antes de esos cambios si necesitas su registro. Cerrar y volver a abrir la ventana conserva las decisiones mientras el análisis siga vigente. Límite: 512 pares omitidos por análisis.

Comandos de interfaz: `trapping.omitPair {omit?:bool}` (true por defecto) actúa sobre la zona/par explícitos y vigentes; false retira la omisión. `trapping.nextDevelopmentPair {}` busca el siguiente par sin omitir. No decide por tintas compartidas. El diagnóstico añade `settings.development.omissions` con estado `omitted`, motivo `operatorDecision`, zona, par, IDs, región y etiquetas; `omissionScope` indica `currentCaptureOnly`. Un informe caducado exporta una lista vacía.

Validación KX27.8: 28 pruebas de interfaz (10 del recorrido y 18 de la paleta), más captura nativa de los estados disponible/omitido/diagnóstico en 1280×800, 1024×700 y 700×560. `cargo check -p vectorcraft-web --target wasm32-unknown-unknown`, formato, assets, brands y layers pasan. La revisión visual independiente dio `ship`; en el tamaño compacto, el mapa requiere desplazamiento del cuerpo para verlo completo. Clippy de la UI no añade avisos a los previos de Pathfinder y miniaturas; `cargo xtask ci` se detiene en avisos existentes de `render/src/fx.rs` y `pdf/src/import_scan.rs`, por lo que no se declara un pase global de CI.

## Uso

1. Abre **Window / Ventana → Trapping**. También está en el menú del panel Pathfinder: **Trap…**.
2. Selecciona trazados sin contorno: sólidos de proceso/spot, sólido contra degradado lineal opaco o sólido contra imagen PNG/JPEG opaca incrustada.
3. Define ancho (mm), dirección, tratamiento del negro y diferencia mínima. **Analizar contactos** permite revisar los encuentros previstos sin modificar el documento; **Aplicar a la selección** crea o recalcula los traps.
4. La paleta permanece abierta. En **Traps de la selección**, ajusta ancho y dirección de cada par y pulsa **Actualizar par**.
5. Al recalcular, incluye el arte original y sus traps. **Restablecer pares** reemplaza los ajustes individuales con las reglas generales.

La paleta puede flotar o acoplarse como los demás paneles. Su ubicación se guarda con los espacios de trabajo; sus parámetros se serializan en las preferencias de interfaz. Las reglas nuevas se guardan en los atributos `data-kx27-trap-v1-*` del objeto, separadas de los nombres visibles y asociadas al ID del trap. Acompañan las copias del objeto sin cambiar el formato nativo. Se leen también los nombres de controles del plugin 0.4; al recalcular se migran a metadatos. El plugin WASM original no interpreta estas reglas nuevas. Aplicar o actualizar un par es un solo paso de deshacer.

**Localizar** selecciona un trap en el lienzo. Para volver a calcular, selecciona de nuevo el arte y sus traps. Los pares mostrados pertenecen a la selección actual.

## Captura experimental por tintas: fase 1

En **Captura por tintas · Fase 1**, elige una tinta, define los ppp y pulsa **Capturar tinta**. Se captura la mesa activa completa, sin selección ni controles de edición. Negro representa tinta y blanco ausencia de esa tinta. La ampliación y el desplazamiento permiten inspeccionar el mapa; ampliar no añade resolución. No es un análisis de conflictos y no crea, cambia ni elimina traps.

- Comando de consulta `trapping.capture`: `{plate?, dpi?:36..600 (150), artboard?:index (0), region?:[x0,y0,x1,y1]}`; coordenadas en puntos. Devuelve PNG base64, tamaño, resolución, milímetros por píxel, región, lista de planchas, ajustes de color y advertencias explícitas. Una región pequeña permite mayor resolución dentro del presupuesto; el panel inicial captura solo la mesa completa.
- El cálculo usa el documento compuesto y todos sus oclusores, aunque no estén seleccionados o estén bloqueados. Excluye guías, capas ocultas, plantillas y capas no imprimibles; no excluye automáticamente troqueles visibles imprimibles ni traps existentes. Ignora la vista de contornos y el dimming de imágenes de las capas, sin cambiar esas opciones en el documento.
- Las tintas planas requieren muestras spot vinculadas. Un SVG que convirtió Pantone a RGB no permite reconstruir su identidad. El alternativo de un spot no se vierte en las planchas de proceso. RGB y Lab se convierten con los ajustes de color activos.
- Las imágenes incrustadas se separan en una copia temporal: muestras CMYK originales cuando existen, RGB separado mediante LUT del perfil activo, sin contribución a spots. Se conserva alfa; los píxeles opacos sin tinta en una plancha aún calan la tinta inferior, y los transparentes la dejan pasar. Recortes y máscaras vectoriales de opacidad siguen el renderizador; las imágenes dentro de máscaras de opacidad se rechazan para no recolorear su luminancia por error.
- Presupuesto independiente de Apply: 4 MP de salida, 32 MP de imágenes fuente acumulados, 64 MiB codificados, 16.384 px por lado de imagen fuente, 20.000 nodos y 128 niveles. Se revisan cabeceras antes de decodificar y se limita memoria de decodificación. Conservadoramente se cuentan también los assets no utilizados. No se baja la resolución silenciosamente. El panel también rechaza subir una textura que exceda el límite gráfico comunicado por el dispositivo y pide bajar los ppp. Imágenes dañadas, ausentes, vinculadas o proxy devuelven error.
- Primera versión: efectos y patrones, símbolos/documentos colocados y otros objetos vivos deben expandirse en una copia antes de capturar. La sobreimpresión usa la aproximación Multiply del renderer existente, no todos los modos PDF de sobreimpresión. Mezclas, interpolación de degradados, antialiasing y conversiones de color son los de una previsualización de 8 bits: no es un RIP ni una referencia contractual de separaciones.
- A 150 ppp un píxel mide aproximadamente 0,169 mm: una captura de página completa **no permite medir o garantizar un trap de 0,10 mm**. Se necesita una región pequeña a mayor resolución y validación posterior para estudiar detalles finos. La captura puede tardar varios segundos y esta primera consulta es síncrona; no tiene progreso ni cancelación.
- El panel conserva una sola captura en memoria, sin escribirla en el documento o preferencias. Se invalida al cambiar documento, mesa, tinta, resolución o ajustes de color. Los clics de selección y la ampliación no fuerzan un nuevo render. No usa IA ni envía el archivo a un servicio externo.

La fase 2 descrita a continuación añade candidatos a partir de estas planchas. La fase 3 inicial añade atribución por muestreo a sólidos originales. Todavía falta verificar el contacto con las curvas y decidir si requiere trapping. **Aplicar trapping mantiene su límite de 80 objetos y sus restricciones actuales.**

## Fronteras visibles: fase 2

Abre **Fronteras visibles · Fase 2** en la misma paleta. Comienza con **72 ppp** y **15 %** de contraste; pulsa **Detectar fronteras** sin seleccionar objetos. El mapa muestra el trabajo compuesto con candidatos rojos y un halo blanco para distinguirlos sobre tintas oscuras. **Mostrar marcas rojas** compara el original y el mapa; ampliar y desplazar solo cambia la vista del mapa. La lista de zonas muestra las tintas que cambian, diez zonas por página. Al elegir una fila se dibuja su recuadro aproximado y se seleccionan los sólidos originales atribuibles, sin editar el arte ni su historial. También puedes pulsar una marca directamente en el mapa.

### Qué detecta

`trapping.boundaries` es una consulta independiente de `trapping.analyze` y `trapping.apply`. Captura todas las planchas CMYK y las muestras spot del documento con la misma región y resolución. El detector original compara valores de tinta, no colores RGB: dos spots con el mismo alternativo visual siguen siendo dos canales distintos.

En cada eje usa cinco muestras consecutivas: exige que una tinta aumente y otra disminuya al menos el contraste elegido entre los extremos. Los cambios centrales deben dominar los exteriores para descartar rampas suaves. Tres muestras internas deben contener tinta para no unir colores separados por un hueco blanco resuelto. El margen exterior de dos píxeles no se analiza. Los píxeles candidatos se agrupan por conectividad de ocho vecinos; fragmentos de menos de cuatro píxeles se descartan por defecto. Una zona conectada puede contener varios encuentros y varias tintas; no equivale a un par de objetos ni a un trap.

### Contrato y límites

- Consulta: `{dpi?:36..600 (72), artboard?:index (0), region?:[x0,y0,x1,y1], contrast?:5..100 (15), minPixels?:2..1000 (4)}`. Las regiones y los recuadros devueltos están en puntos del documento. Un recorte pequeño a mayor resolución se puede pedir por el comando; el panel inicial analiza la mesa activa completa.
- Devuelve `dataBase64` (PNG compuesto), `overlayBase64` (PNG transparente de marcas), tamaño, ppp, mm/píxel, ajustes de color, planchas, `zoneCount`, `markedPixels`, `discardedPixels` y `zones:[{index,pixels,bounds,plates,attribution}]`, `sourceObjects` y `zoneMapBase64` (PNG interno para localizar zonas al pulsar el mapa). `index` es un número local del informe, no un ID del documento. El informe enumera solo las primeras 200 zonas en orden de escaneo y declara `zonesTruncated`; el overlay conserva **todas** las marcas retenidas, también las de zonas no enumeradas.
- Máximo **1 MP de salida, 12 tintas totales (CMYK incluidas) y 8 MP de imágenes fuente acumulados**. Las imágenes fuente y el resto del arte heredan también las validaciones y límites de fase 1. Se incluyen conservadoramente las muestras spot y assets no utilizados en sus respectivos presupuestos. Los límites se revisan antes de renderizar cada plancha; no se baja la resolución silenciosamente. El panel verifica el límite de textura del dispositivo antes de subir ambas imágenes.
- Se mantienen oclusores, recortes, alfa y capas imprimibles visibles, incluidas las bloqueadas. Los troqueles imprimibles visibles y los traps existentes también aparecen: no se clasifican ni excluyen automáticamente. No hay filtrado por selección ni se aplica el límite de 80 objetos a esta consulta.
- Una sola pareja de texturas, un mapa de índices de zona y un informe se conservan en memoria. Se invalidan al cambiar documento, mesa activa, resolución, contraste o ajustes de color. Cambiar selección, mostrar/ocultar marcas, ampliar, paginar o revisar una zona no recalcula ni modifica el documento. No se guarda el mapa en el archivo o preferencias; solo sus controles se serializan.

### Qué no concluye

Es un detector conservador para **revisión**, no una confirmación de conflicto ni una decisión de trapping. Omite deliberadamente tinta contra papel, cambios de una sola plancha, gradaciones suaves y fragmentos pequeños; puede perder contactos finos, bordes recortados por el margen o cambios inferiores al contraste. El aliasing puede ocultar huecos menores que un píxel; los detalles de fotografías y mezclas pueden generar falsos positivos. A 72 ppp un píxel mide 0,353 mm: el rojo, su halo y los recuadros son indicadores de pantalla, **no el ancho de un trap**.

Hereda la aproximación de sobreimpresión Multiply y el renderizador de 8 bits de fase 1; no es una referencia RIP. No recupera spots convertidos a RGB/CMYK, la atribución inicial se limita a sólidos opacos y no valida el contacto vectorial; no decide necesidad/dirección/ancho y no crea vectores. El cálculo es síncrono y repite la separación de imágenes por plancha, sin progreso ni cancelación: esta entrega no afirma una mejora de tiempos frente a un motor comercial. No usa IA ni servicios externos. **Aplicar trapping conserva sus límites y reglas actuales.**

La comprobación local de curvas descrita a continuación permite revisar muestras de estos pares antes de preparar traps.

## Relacionar zonas con originales: fase 3 inicial

Después de **Detectar fronteras**, pulsa una marca roja o una fila de zona. KX27 selecciona los objetos originales de sus pares atribuibles y muestra **Objetos de la zona**. Una zona conectada puede reunir varios pares: **Resaltar par N** selecciona únicamente los dos objetos de ese par, sin desplazar la vista. Si no hay pareja atribuible conserva la selección anterior y explica la limitación. Esta revisión cambia solo la selección: no crea objetos ni pasos de deshacer y no aplica trapping.

El algoritmo añade **un render temporal de identificadores**, con bordes sin suavizado, a la misma región y resolución de las planchas. No convierte los originales a nuevos contornos ni compara todos los vectores entre sí. Mantiene las curvas, reglas de relleno, agujeros, orden de apilado y recortes admitidos. El color auxiliar se expresa mediante una conversión Lab analítica para que el perfil RGB de trabajo no recorte ni cambie los códigos; codifica una referencia temporal; los IDs del informe son los IDs originales del documento. Las tintas de cada objeto provienen de sus pinturas y muestras vinculadas, no del RGB auxiliar.

En cada una de las primeras 200 zonas utiliza hasta **256 sondeos**, distribuidos por sus píxeles reales y por los ejes donde se detectó intercambio de tinta. Los cinco puntos de cada sondeo deben pertenecer únicamente a los dos sólidos de sus extremos; papel, un tercer objeto o un bloqueador dejan el sondeo sin atribuir. Se agrupan hasta **64 pares por zona**, con recuentos de muestras y truncado explícito. La atribución declara `resolved`, `partial` o `unresolved`: **resolved significa que todos los sondeos elegidos se atribuyeron, no que toda la frontera esté verificada**. Puede omitir pares muy pequeños entre muestras y asociar superficies separadas por huecos inferiores a la resolución.

Admite trazados y compuestos con un único relleno sólido opaco, de proceso o spot, sin trazo activo, sobreimpresión, máscara ni mezcla especial. Los grupos normales y sus recortes simples se conservan. Imágenes, degradados, trazos, máscaras, grupos con composición especial y traps identificados por metadatos bloquean conservadoramente su caja en el mapa auxiliar; no se mira a través de ellos para inventar una pareja inferior. Un recorte cuyo contorno requiera composición no admitida bloquea el grupo completo. Esto puede dejar zonas sin atribuir o atribuirlas parcialmente. No clasifica automáticamente troqueles o tintas técnicas.

El clic usa los píxeles de la zona y una tolerancia de dos píxeles, no su caja rectangular. Empates entre zonas se ignoran; la lista permite elegirlas expresamente. Las zonas posteriores al límite de 200 conservan sus marcas visuales, pero no se pueden localizar por clic ni tienen atribución detallada. La UI usa `app.run("select.set", ...)`; no añade un comando de edición ni modifica el cálculo de Apply. El informe se invalida con documento/mesa/ppp/contraste/ajustes de color, igual que el mapa. Cambiar selección no lo recalcula.

El JSON incluye `sourceObjects` (ID, nombre acotado, tintas, caja) y la atribución de cada zona cuando el informe está vigente. Omite el PNG interno `zoneMapBase64`, igual que los otros PNG: no incrusta imágenes ni geometría completa. Sigue siendo diagnóstico de la sesión, no un archivo importable de traps.

A 72 ppp no se mide un trap de 0,10 mm. Antes de generar trapping a partir de estos pares hay que comprobar el borde visible exacto con la geometría original, su oclusión y las reglas de tinta. **El límite de 80 objetos y las restricciones de Aplicar permanecen vigentes.**

## Verificar curvas locales

Después de **Detectar fronteras**, elige una zona y pulsa **Verificar curvas del par N**. No hace falta seleccionar todo el trabajo. El resultado indica cuántas muestras de frontera visible se encontraron, cuántas quedaron tapadas, cuántas encontraron papel y cuántas resultaron inciertas. **Verde** marca puntos sobre la curva original dentro del mapa. Puedes ampliar el mapa y ocultar las marcas rojas para revisarlos. Las marcas no representan un trapping de 0,10 mm ni se convierten en objetos del documento.

La consulta `trapping.verifyCurves` recibe `{ids:[idA,idB],region:[x0,y0,x1,y1]}` en puntos. Trabaja con la pareja indicada y los contornos originales de sus recortes, no con los píxeles rojos ni con una comparación de todos los vectores entre sí. Reutiliza la escena conservadora de atribución: sólidos opacos sin trazo/sobreimpresión se comprueban con sus reglas reales de relleno y apilado, incluidos agujeros, cierres implícitos y recortes simples. Las demás superficies bloquean su caja y producen incertidumbre cuando intervienen; un recorte no admitido bloquea el grupo completo.

Evalúa directamente segmentos lineales, cuadráticos o cúbicos. Subdivide las curvas locales hasta que la diagonal de cada caja mida como máximo **0,05 mm**. En el punto medio prueba ambos lados de la normal a **0,0001 mm** y a **0,002 mm**; exige encontrar los mismos dos objetos en el mismo orden en ambas escalas. Es una comprobación numérica con tolerancia, no un cálculo exacto de distancia entre curvas ni una prueba de toda la frontera. Los encuentros entre muestras y los huecos inferiores a la tolerancia pueden perderse. La precisión matemática declarada no equivale a resolución de impresión, precisión del archivo o garantía de registro.

Devuelve `testedSamples`, `visibleSamples`, `occludedSamples`, `gapSamples`, `unresolvedSamples`, `truncated`, `localObjects`, `stepMm`, `probeMm` y `status`:

- `visibleBoundaryFound`: hay muestras atribuibles a ambos lados y terminó el recorrido sin incertidumbre; **no significa que todo el borde esté comprobado**.
- `noVerifiedBoundary`: no encontró esas muestras en el recorrido; no demuestra ausencia de contacto entre ellas.
- `incomplete`: encontró composición incierta o alcanzó un presupuesto. Puede incluir puntos verdes válidos, pero nunca confirma ausencia de contacto.

`fragments` contiene inicio/medio/fin de pequeños fragmentos de curva en puntos del documento (`[ax,ay,mx,my,bx,by]`). **Solo el punto medio se clasifica**, declarado como `fragmentVerification:"midpointOnly"`. La UI marca ese punto con un halo y un centro verde legibles incluso cuando el fragmento es menor que un píxel de pantalla. Se muestran hasta 1.024 fragmentos distribuidos por el resultado, con `fragmentsTruncated` si se resumió el mapa; este resumen no cambia los recuentos completos de muestras evaluadas.

Presupuestos: hereda 20.000 nodos/128 niveles de la captura; hasta **512 superficies locales y 8.000 anclas contabilizadas** entre rellenos y recortes. El número de sondeos se ajusta al coste de las superficies/recortes, con un máximo de **8.192**; hasta 60.000 visitas de subdivisión y profundidad 24. Un presupuesto de geometría o superficies devuelve error con indicación de usar una región menor; los límites durante el muestreo dejan `truncated:true` y estado incompleto. El panel usa la caja de la zona; una región menor se puede pedir mediante el comando. No decodifica imágenes ni ejecuta offsets o booleanas de pares globales.

La consulta y el botón no cambian documento, selección, historial o muestras de tinta. El informe se conserva solo para la captura y zona que lo originaron; cambiar zona, volver a detectar o invalidar el mapa lo descarta. El JSON comprueba su vigencia incluso antes del siguiente frame y añade `analyses.curves` y `localCurveSamplesIncluded`. Los fragmentos son muestras locales, no contornos completos: se mantiene `fullGeometryIncluded:false`.

Esta etapa todavía **no genera trapping desde el mapa** ni modifica el límite de 80 objetos de Aplicar. El siguiente paso es preparar geometría de trap local con reglas de tinta y una previsualización independiente, sin convertir estas muestras en una validación completa.


## Previsualizar una franja local

Después de verificar un par con muestras visibles aparece **Previsualización local de trapping**. El ancho inicial es **0,10 mm** (rango inicial 0,01–0,50 mm). **Origen** identifica la tinta que se expandiría; **Receptor** identifica el objeto hacia el que entra. **Invertir origen y receptor** cambia esa dirección explícita: no aplica automáticamente la regla claro/oscuro del botón Aplicar. Pulsa **Previsualizar franja**, amplía el recorte con **Ampliar franja** y desplázate en ambos ejes. **Mostrar franja resaltada** permite comparar la geometría temporal con la vista original. El resaltado usa el acento del tema: **no representa el color de sobreimpresión**. La imagen base conserva los colores de los sólidos soportados; composiciones desconocidas se muestran como bloqueadores grises.

La consulta `trapping.previewLocal` recibe `{ids:[origen,receptor],region:[x0,y0,x1,y1],widthMm:0.1}`. Trabaja localmente sobre líneas, cuadráticas y cúbicas originales y contornos de recorte. Cada intervalo de diagonal máxima 0,05 mm exige que cinco posiciones interiores encuentren el mismo par a ambos lados, a 0,0001 mm y 0,002 mm. Agrupa intervalos adyacentes de **la misma curva original por sus parámetros**, sin unir los puntos verdes del diagnóstico. Conserva las curvas al formar los tramos; el offset del stroker y los booleanos son aproximaciones numéricas, no un offset Bézier exacto. El ancho nominal usa contorno doble recortado al receptor, uniones redondas y extremos rectos. Las comprobaciones siguen siendo muestreadas: pueden perder detalles entre sondeos y acortar extremos; no prueban toda la frontera continua.

El receptor se normaliza con su regla de relleno y sus recortes, y se restan las superficies superiores, incluidas las cajas conservadoras de composiciones desconocidas. Así se limita **todo el ancho** de la franja, no solo sus muestras centrales. Los huecos no se rellenan. Se amplía la región solicitada con un pequeño margen de revisión; sus cortes son límites de análisis, no extremos definitivos del contacto. En esquinas y zonas estrechas el recorte puede reducir el ancho efectivo. Se conserva la pintura enlazada del origen, incluidos spot/tint, como metadata `sourcePaint` y la intención `plannedOverprint:true`; **previsualizar no crea rellenos de sobreimpresión en el documento**. La incorporación explícita se describe a continuación.

Presupuestos iniciales: hasta 32 superficies y 1.024 anclas de entrada locales (incluidos recortes únicos), 256 tramos separados, 4.096 anclas de salida, 60.000 visitas de subdivisión y hasta 8.192 intervalos, con presupuesto de coste de pertenencia. Al alcanzarlos se devuelve un error recuperable para elegir una zona menor, sin una franja parcial. Las dos imágenes temporales tienen como máximo 1.000 píxeles por lado y 600 ppp; la resolución real se informa en `dpi`, puede ser menor y no constituye una medición de impresión. La UI comprueba también el límite de textura del dispositivo. No se decodifican imágenes originales para esta vista; se representan como bloqueadores.

La respuesta incluye `status:"previewOnly"`, un `previewId` de sesión, IDs ordenados, ancho, región solicitada/efectiva, tramos originales, intervalos probados, `bandPath` local en sintaxis SVG, límites/anclas y dos PNG internos para la vista. El JSON de diagnóstico añade `analyses.preview` y `localPreviewGeometryIncluded`; elimina ambos PNG. Solo exporta la franja vigente para el mismo documento, CMS, zona, verificación, ancho y dirección. Cambiar un control de cálculo o el documento invalida la franja; zoom, ocultar el resaltado y seleccionar objetos no la recalculan. La geometría del documento, selección y deshacer/rehacer se conservan.

**Previsualizar no sustituye Aplicar a la selección ni incorpora la franja por sí solo.** Aplicar conserva el cálculo anterior y el límite de 80 objetos; **Incorporar esta franja** utiliza la geometría local revisada. La prueba de contacto continuo, reglas automáticas, revisión en RIP y extensión de este recorrido a degradados/imágenes pertenecen a etapas posteriores. No se afirma paridad de trapping comercial ni mejora de rendimiento medida.


## Incorporar la franja revisada

Después de **Previsualizar franja**, revisa el ancho y la dirección y pulsa **Incorporar esta franja**. Crea **un trazado vectorial** con la geometría preparada, en la capa superior **KX27 · Trapping local**, con relleno de la tinta de origen y sobreimpresión activada. Conserva los enlaces spot y sus porcentajes; no convierte Pantone a su color alternativo. Los originales mantienen sus nodos y atributos. La selección pasa al nuevo trap. **Edición → Deshacer** revierte la creación de franja/capa en un solo paso; Rehacer la restaura.

`trapping.previewLocal` guarda una única franja preparada, acotada, en la sesión del motor y devuelve `previewId`. La sesión conserva una referencia débil al documento, su UID y ajustes CMS, la geometría y pintura exactas. `trapping.incorporateLocal {previewId}` **no admite una ruta SVG, pintura ni coordenadas del llamador**, no vuelve a calcular el offset ni importa geometría del JSON: consume esa franja exacta. Cambiar el documento, cambiar CMS, abrir otra sesión o generar otra previsualización invalida el ticket anterior; selección y zoom no lo invalidan. Si se vuelve por Deshacer al mismo documento compartido sin haber consumido el ticket, sigue siendo la misma geometría. Una incorporación exitosa consume el ticket; repetirlo no crea otro objeto. Los tickets no se guardan en archivos y el diagnóstico no permite restaurarlos.

Antes de incorporar exige terminar interacciones, grupos de deshacer y edición de máscaras/aislamiento. La capa gestionada se identifica por metadata, admite renombrarse y se reutiliza para otros pares. Debe estar arriba, visible, desbloqueada, imprimible, en vista normal, sin recortes, efectos ni composición especial. Si se cambió, devuelve una instrucción recuperable; no la reordena ni corrige silenciosamente. La inserción y la selección forman una transacción; cualquier fallo mantiene documento e historial.

La etapa inicial incorpora pares de **CMYK o muestras spot vinculadas convencionales**. RGB/Gris/Lab de proceso requieren conversión antes de volver a previsualizar. La pintura de origen se mantiene exactamente: si algún componente no nulo reduciría la cobertura de una tinta compartida con el receptor, se rechaza esa incorporación; copiarla sin mezclar coberturas podría aclarar el receptor. Revisa la dirección. La mezcla específica de tintas compartidas queda pendiente. No se incorpora una pintura sin cobertura. Registro, blancos opacos y tintas técnicas siguen fuera del alcance inicial.

Cada trap guarda `kx27-local-trap-v1` con origen/receptor, ancho, región solicitada y nombres; la capa guarda `kx27-local-trap-layer-v1`. La comprobación recorre metadata existente, incluidos traps ocultos, y rechaza un segundo trap para el mismo par (en cualquier dirección) cuya región solicitada se solape. Es conservadora: para sustituirlo elimina/deshaz el anterior y genera otra vista. No actualiza automáticamente franjas editadas ni une regiones. La geometría nativa y el enlace de pintura se guardan por el formato normal; la metadata acompaña el archivo nativo. Los traps locales se excluyen de las fuentes atribuibles, incluso si se desactiva su sobreimpresión; se conservan como bloqueadores conservadores, por lo que pueden limitar un análisis posterior en su zona.

**Traps de la selección** identifica la **Franja local incorporada** y su estado de sobreimpresión; `trapping.info` y el snapshot del diagnóstico añaden `localTraps`. El comando anterior **Aplicar a la selección** no recalcula estos objetos: los rechaza expresamente para evitar mezclar ambos métodos. Tras incorporar se descartan los mapas, curvas y previsualización que pertenecían al documento anterior. El JSON posterior conserva la información del trap seleccionado y sus pinturas, con los análisis descartados como `notAvailable`.

La incorporación sigue siendo **experimental y local**: no elimina los límites del muestreo ni confirma la necesidad del trap. Puede recortar extremos de contacto y detalles entre sondeos. Guardar una tinta Separation y los operadores de sobreimpresión en PDF no constituye una validación de salida en RIP. La vista de sobreimpresión del editor conserva sus aproximaciones anteriores. No se afirma cobertura completa de tintas DeviceN ni equivalencia a trapping comercial.

## Exportar diagnóstico JSON

El botón **Exportar diagnóstico (.json)** está al inicio de la paleta y funciona con un documento abierto, incluso si Aplicar falló o no hay selección. En la web descarga `nombre-trapping.json`; en escritorio abre el diálogo de guardado con filtro JSON. Si una ruta explícita o el diálogo devuelve otra extensión, se añade `.json` para conservar el documento fuente. No ejecuta el cálculo de trapping ni vuelve a generar capturas/fronteras. No cambia el documento, la selección, el historial ni el error previo mostrado por la paleta; la confirmación de exportación es independiente.

El comando de interfaz `trapping.exportDiagnostic {path?}` usa los mismos servicios de descarga/escritura y el sistema de diálogos asíncronos de la app. Devuelve `{path,bytes,schemaVersion}`. Requiere documento; los errores de ruta, cancelación o escritura no producen una falsa confirmación. No es un comando del motor headless, porque los informes se conservan en la interfaz.

Formato `kx27-trapping-diagnostic`, `schemaVersion:1`:

- Versión de la app y commit de compilación cuando `GITHUB_SHA` está disponible; en compilaciones locales puede ser `null`.
- Documento activo, revisión/ID de sesión, modo de color, formato de origen y hasta 100 mesas; ajustes de color y lista de planchas de proceso/spot.
- Controles de trapping, captura, detector y previsualización local; último estado/mensaje de la paleta y snapshot de selección/reglas existentes. Si este snapshot falla, se conserva su error y el resto del diagnóstico se puede exportar.
- Muestra de hasta 200 elementos nativos, con ID, nombre, tipo, atributos locales y hasta 16 pinturas por elemento. Los sólidos conservan modelo/componentes, enlace de muestra, tint y sobreimpresión; los degradados resumen hasta 16 paradas. Se recorren como máximo 20.000 nodos y 128 niveles, con indicadores de truncado. Los grupos/capas y el arte interno de máscaras/símbolos no constituyen un inventario completo de superficies.
- Informes ya calculados de contactos, captura de una tinta, fronteras, curvas y previsualización local. Cada uno declara `current`, `stale` o `notAvailable`; los informes obsoletos se omiten. Se comprueban documento/UID, controles pertinentes, mesa y ajustes de color incluso si el comando se invoca antes del próximo frame. `notAvailable` también puede significar que un informe se invalidó y se descartó previamente.

Se excluyen expresamente PNG/base64 y texturas de los informes. `fullGeometryIncluded:false` e `imageBytesIncluded:false`: puede contener recuadros y muestras locales de curvas originales, pero no contornos completos del arte original ni píxeles del diseño. Si `localPreviewGeometryIncluded:true`, también contiene el contorno local temporal `bandPath` de la franja previsualizada, identificado expresamente como geometría de revisión y no como un objeto incorporado al documento. El JSON no reemplaza el SVG/PDF/archivo nativo y no es un preset importable. Para reproducir un fallo completo se necesita también el original. La salida se limita a 8 MiB antes de escribir; el usuario decide cuándo compartirla.

La consulta de contactos ahora se invalida también cuando cambian los ajustes de color, igual que las capturas y las fronteras. Los informes de captura/fronteras conservan sus advertencias experimentales, su truncado y sus límites; exportarlos no confirma necesidad de trapping ni constituye validación RIP.

## Revisar antes de aplicar

**Análisis de contactos** muestra una fila por pareja de objetos fuente: nombres de los trazados, tintas, dirección, ancho y motivo (claridad, negro K, diferencia mínima o ajuste existente). Los encuentros sólido/sólido de las mismas tintas comparten regla y trap; los de degradado se asocian al objeto de degradado y al color sólido; todavía no hay excepciones por borde ni edición local. Componentes separados entre los mismos dos objetos aparecen en una sola fila.

**Seleccionar pareja** selecciona los dos objetos fuente sin desplazar la vista. **Recuperar selección analizada** restaura el ámbito original; aplicar queda deshabilitado mientras se inspecciona una selección distinta. El informe persiste al cambiar la selección y se descarta al editar el documento, cambiar sus reglas, aplicar o cambiar de documento. Es temporal y no se guarda en preferencias ni en el archivo.

El informe y Aplicar comparten un plan geométrico puro. Analizar calcula recortes y offsets temporales para prever las mismas zonas, pero no materializa objetos, cambia la selección ni crea pasos de deshacer. Cero encuentros es un resultado válido de análisis. El ámbito, ancho y tolerancias del algoritmo determinan los encuentros previstos: puede incluir separaciones pequeñas dentro del ancho; no es un detector universal de adyacencia ni un motor completo de visibilidad. Es el primer paso de revisión separada, no la implementación completa de las cuatro etapas profesionales.

## Regla inicial: sólido hacia degradado

En un encuentro entre un sólido y un degradado lineal opaco, el sólido se expande hacia el degradado con el ancho elegido durante toda la unión. El trap lleva solo la tinta del sólido, en sobreimpresión, sin degradado propio. Esta regla fija prevalece sobre dirección general (automática, invertida o centrada), diferencia mínima y tratamiento de negro. En el par puedes cambiar ancho o elegir **Sin trap**; no invertir, centrar ni sobreimprimir toda la superficie del negro.

Los cuatro casos iniciales son amarillo hacia degradado cian, amarillo hacia degradado magenta, magenta hacia degradado cian y cian hacia degradado magenta. Se prueban con ambos órdenes de apilado. El motor usa la geometría visible y no toca las paradas, ubicación, porcentajes ni pintura de los originales. El blanco de proceso al 0 % se admite como parada opaca: no equivale a una parada transparente. No se detecta ni invierte la dirección por claridad local; tampoco se decide por un color promedio.

**Analizar contactos** identifica la regla fija. Cada degradado utiliza una identidad interna por ID de objeto, mostrada como **Degradado · nombre del objeto**; no se muestran claves internas. El trap conserva su ID, ancho y exclusión al recalcular con el mismo arte. Duplicar el degradado crea otra identidad y requiere revisar sus ajustes. Guarda el archivo nativo para conservar las reglas; importar un PDF de producción no sustituye ese archivo.

Admite entre 2 y 256 paradas ordenadas de proceso y plenamente opacas, con geometría válida. Un encuentro visible entre dos degradados se rechaza de forma atómica; varios degradados separados en la misma selección sí son posibles. Los degradados opacos también ocultan los encuentros que queden debajo. La regla no introduce rasterización. El RGB del SVG conserva la conversión simple existente; el PDF CMYK utiliza sus tintas de proceso. No se garantiza idéntica separación a partir de RGB y CMYK que solo se parezcan en pantalla.

## Primera etapa de imágenes: sólido hacia marco opaco

Coloca una imagen con **File → Place**, sin Link (o incrústala antes). Selecciona la imagen y el sólido, sin textos ni otros objetos vivos; usa **Analizar contactos** y luego **Aplicar a la selección**. Se admiten PNG y JPEG incrustados, con todos sus píxeles plenamente opacos. Un PNG con canal alfa también sirve si todos sus valores son exactamente opacos; se comprueban tanto alfa de 8 bits como de 16 bits. El blanco dentro de una foto opaca sigue perteneciendo a su marco.

El sólido entra en la imagen con ancho nominal constante, solo su tinta y sobreimpresión. Esta regla fija prevalece sobre dirección global, diferencia mínima y tratamiento de negro; el par permite ancho o **Sin trap**. El informe dice **Sólido hacia imagen opaca** y muestra **Imagen · nombre**. La identidad interna se asocia al ID de imagen; duplicarla crea un par nuevo que requiere revisión.

La imagen conserva tipo de objeto, ID, píxeles codificados, perfil embebido, dimensiones, transformación y lugar de apilado. El cálculo recibe exclusivamente un polígono temporal de cuatro esquinas, transformadas con la matriz real de la imagen: también admite giro, espejo e inclinación, sin sustituirlos por una caja alineada a los ejes. Los traps nuevos quedan encima del arte seleccionado; los ya existentes mantienen su ubicación. La integración no remuestrea, vectoriza ni reescribe la imagen. La exportación sigue a cargo del motor PDF existente; se comprueban las referencias de imagen y la sobreimpresión del trap.

Esta etapa **no lee los colores del borde para decidir la dirección**, no sigue siluetas, no compensa transparencias y no calcula imagen contra imagen ni imagen contra degradado. Estos encuentros visibles se rechazan antes de editar. Una imagen opaca sí participa como oclusor de encuentros inferiores. Se rechazan imágenes vinculadas o representadas solo por su vista previa, máscaras, recortes, efectos, apariencias añadidas, calco y perspectiva vinculada. No incluye TIFF ni otros formatos en esta primera etapa.

El examen de opacidad se hace al analizar o aplicar, nunca en cada frame. Por selección: hasta **32 megapíxeles y 64 MB codificados de imágenes distintas**, con decodificación limitada a 128 MB por imagen; una imagen compartida por varios objetos se comprueba una vez. Se valida formato, dimensiones y transformación antes de crear geometría. Datos ausentes, corruptos, excesivos o con transparencia fallan sin cambios de documento, selección ni historial.

Los filtros de plugins siguen recibiendo solo trazados. El adaptador nativo puede conservar referencias de imagen como `{id}` en el orden de pintado; el adaptador de salida exige que esas referencias no incluyan cambios de geometría, nombre, rellenos ni opacidad. Esto evita convertir accidentalmente una imagen en un trazado vacío. No se guardan nuevos campos en el formato nativo.

## Arquitectura y procedencia

- `crates/trapping`: lógica independiente de interfaz adaptada de `kartonpapers-ai/steppa-trap` 0.4.0, `src/trap.rs`, blob `349ab24379f7e4b9577401555e5b17aa95553ae7` (Steppa; MIT OR Apache-2.0). Conserva las 19 pruebas originales y añade pruebas de identidad spot, porcentajes y claves de pares. Añade tolerancia de redondeo f32 en los umbrales de negro del host. No incluye el wrapper ABI ni usa código inseguro.
- `crates/engine/src/cmd/trapping.rs`: `trapping.info` (consulta sin calcular geometría), `trapping.analyze` (plan de encuentros de solo lectura) y `trapping.apply` (validación, cálculo y edición transaccional).
- `crates/ui-egui/src/panels/trapping.rs`: controles, mensajes y ajustes de pares; llama a los mismos comandos que automatización.

Es el mismo motor para escritorio y WebAssembly. No hace falta instalar el `.wasm` del plugin. La integración usa el adaptador de objetos existente y no modifica el contrato de plugins.

## Límites conservados y protecciones

- Rellenos sólidos CMYK, RGB, gris, Lab y muestras globales o spot. También sólido contra degradado lineal opaco de proceso. RGB conserva la conversión simple del plugin; Lab de proceso pasa a CMYK con la configuración de color activa.
- Las muestras spot conservan nombre, vínculo y porcentaje. Su alternativo Lab/CMYK sirve para claridad y pantalla, nunca se suma como tinta CMYK. Dos spots distintos con el mismo alternativo siguen siendo dos planchas. Registro y nombres de colorante reservados por PDF (All, None, Cyan, Magenta, Yellow, Black) se rechazan. No se incluye una biblioteca de valores Pantone.
- Rechaza contornos, transparencias, efectos y grupos con recorte o máscara en los objetos procesados. Los degradados radiales/libres, los vinculados a una muestra de degradado y los degradados con paradas spot se rechazan en esta etapa. No calcula sobre texto ni objetos vivos. Las imágenes solo se admiten en la etapa acotada descrita a continuación.
- Los traps llevan rellenos independientes en sobreimpresión: un CMYK de máximos por canal y un relleno por spot presente, con el máximo porcentaje de esa tinta. El exportador existente escribe spots como `Separation` y preserva `OP/op` y `OPM`; no se añade una pintura DeviceN al modelo. Exporta con sobreimpresión **Preserve**, sin convertir spots a proceso. View → Overprint Preview aproxima las mezclas en pantalla con Multiply; no es una prueba colorimétrica ni equivale a un RIP.
- Las curvas se convierten a segmentos con el método y los límites de subdivisión del plugin 0.4. No es un offset Bézier exacto.
- Máximo 80 objetos (incluidos traps) y 12.000 nodos de entrada por operación. El cálculo se realiza al pulsar un botón, no en cada frame.
- Incluye todo el arte que pueda ocultar un encuentro. El motor solo conoce el ámbito seleccionado; no hace una comprobación integral del documento.
- Los nombres heredados que empiezan por `trap` se interpretan como controles. Las reglas nuevas se consultan por ID, aunque cambies el nombre visible del trap. Renombrar una muestra spot cambia su identidad de par: al recalcular se aplican reglas nuevas. Revisa los ajustes después de renombrar, duplicar o copiar traps entre documentos.

Blanco opaco, metálicos, barnices y otras tintas técnicas requieren reglas y datos de impresión adicionales; esta etapa no los valida. Se rechazan spots con alternativo blanco a porcentaje positivo. Esta integración no se presenta como trapping profesional validado. Antes de producción, revisar separaciones y archivos exportados con casos de referencia de imprenta.

## Comprobar un par spot

1. Crea una muestra global de tipo Spot (por ejemplo `Tinta prueba A`) y aplica 40 % a un trazado.
2. Hazlo coincidir con un trazado CMYK u otra muestra Spot; selecciona ambos.
3. Aplica Trapping y comprueba los nombres y porcentajes en Appearance. Los rellenos del trap deben marcar sobreimpresión.
4. Guarda y vuelve a abrir el documento nativo; ajusta el ancho de ese par y comprueba deshacer. Conserva el nativo para recalcular: un PDF de producción sin datos de edición no guarda todas las reglas.
5. Exporta PDF con Advanced → Overprint: Preserve y conserva las tintas directas en Output. Verifica las planchas con un RIP de referencia antes de producción.

Referencias públicas de formato: [ISO 32000, espacios de color](https://pdf-issues.pdfa.org/32000-2-2020/clause08.html) y [planos CMYK/spot de referencia](https://ghostscript.readthedocs.io/en/latest/Devices.html#tiff).

## Validación

Curvas locales (2026-10-27): pruebas de bordes compartidos y superpuestos, curvas cúbicas sobre el contorno original, huecos pequeños, oclusión, transparencia como bloqueador, recortes/agujeros, cierres implícitos, spots, documentos con más de 80 objetos, presupuestos y entradas inválidas. La UI prueba verificación, JSON vigente/obsoleto y conservación de documento/selección/deshacer/rehacer. Pasaron las 8 pruebas de consulta y las 16 de paleta (incluida captura nativa de una elipse con comprobación de píxeles verdes en claro/oscuro), además de las 18 de captura/atribución y las 24 de trapping del motor. Pasaron WASM, formato, assets (320), brands y layers (28). El CI general conserva el aviso previo de Clippy en render/fx.rs; los chequeos enfocados conservan los anteriores de Layer Panel, Pathfinder y thumbnail worker. Se revisó la paleta en 280/340 px. No se ejecutó un benchmark comparativo ni se afirma validación de producción.

Fase 3 inicial (2026-10-26): comprobación de atribución a sólidos/spot visibles, exclusión de objetos tapados y composición no admitida, varios pares en una zona, recortes, agujeros, IDs auxiliares superiores a 255, región/ppp y más de 80 objetos. La paleta prueba selección de originales/par, conservación del documento y deshacer/rehacer, invalidación después de editar y omisión del PNG interno en JSON. Pasaron 90 pruebas específicas: 26 de geometría, 24 de trapping del motor, 18 de captura/atribución (dos casos locales opcionales sin activar), 5 del detector, 1 de conversión de los 20.001 códigos auxiliares, 15 de paleta y 1 de localización de píxeles. Pasaron WASM, formato, assets (320), brands y layers (28). Se inspeccionaron capturas nativas claro/oscuro de 280/340 px y se verificó el clic real sobre el mapa. El CI general conserva los avisos anteriores de Clippy en render/PDF; el chequeo enfocado de UI conserva los de Pathfinder/thumbnail worker. No se afirma validación RIP ni una ventaja de rendimiento.

Diagnóstico JSON (2026-10-26): pasaron las 14 pruebas de paleta (9 anteriores y 5 nuevas). Se revisaron capturas nativas en temas claro y oscuro. Pasaron la compilación WASM, formato, assets, brands y layers. El CI general sigue detenido por avisos previos de Clippy en render/PDF. Regresiones de descarga web, guardado de escritorio, extensión JSON, cancelación/fallo de escritura, muestras spot/RGB, informes vigentes/obsoletos, ausencia de imágenes base64 y conservación de documento/selección/historial/error previo. También se prueba el caso real de rechazo por más de 80 objetos con truncado explícito de muestras. Los servicios de archivo se simulan en las pruebas; no se distribuye arte del usuario.

Fase 2 (2026-10-26): pasaron 77 pruebas específicas (26 geometría, 5 detector, 24 trapping del motor, 13 captura/fronteras —dos casos locales opcionales— y 9 paleta). También se aprobaron WASM, formato, assets, brands y layers; el CI general sigue detenido por avisos previos de Clippy en render/PDF y las comprobaciones enfocadas señalan solo los avisos anteriores en Layer Panel, Pathfinder y thumbnail worker. Se inspeccionaron capturas nativas claro/oscuro con el SVG local. El smoke de `vectorcraft-cli bench` antes/después conservó 1.156 nodos dibujados y 31 descartados sobre el trabajo local; los perfiles/cachés de desarrollo difirieron y los tiempos no se usan como afirmación de rendimiento.

Pruebas del detector puro para intercambio brusco, rampas, papel/huecos, tinta común sin intercambio, antialiasing, fragmentos y truncado de informes; pruebas de consulta para recuadros en coordenadas de documento, oclusión, spots de igual apariencia, más de 80 objetos, imágenes CMYK, recortes y parámetros/presupuestos inválidos. El trabajo SVG del usuario se usó solo localmente: 41 zonas, 4.299 píxeles marcados y 74 descartados, a 1.128 × 780 px / 72 ppp / 15 %. Es evidencia de ejecución y conservación del documento, no validación de que esas 41 zonas requieran trap.

```sh
cargo test -p vectorcraft-trapping
cargo test -p vectorcraft-render --lib boundaries::tests
cargo test -p vectorcraft-engine --test trapping
cargo test -p vectorcraft-engine --test trapping_capture
cargo test -p vectorcraft-engine --test trapping_curves
cargo test -p vectorcraft-ui-egui --test trapping_panel
cargo check --target wasm32-unknown-unknown -p vectorcraft-web
cargo xtask ci
```

Actualización de fase 1 (2026-10-25): 26 pruebas de geometría, 24 de trapping del motor, 7 de captura (incluido un caso local opcional) y 8 de paleta. Se comprobó un trabajo SVG del usuario con más de 1.100 objetos: cuatro planchas CMYK de 2.350 × 1.625 px a 150 ppp, sin cambios al documento. Los spots de ese SVG ya estaban convertidos a colores de proceso/RGB y no se reconstruyeron. Las pruebas sintéticas verifican calado y sobreimpresión básica, dos spots de igual alternativo visual separados por identidad, imágenes alfa, muestras CMYK originales, recortes, más de 80 objetos, límites, entradas inválidas y ausencia de cambios en selección/historial/vista. Se revisaron capturas nativas del panel con imagen visible en temas claro y oscuro. No es una validación RIP ni un benchmark web.

Comprobado el 2026-10-25: 26 pruebas de geometría, 24 de motor y 7 de paleta aprobadas; capturas nativas en ambos temas, comprobación WASM, formato, assets, brands y layers aprobados. `cargo xtask ci` se detiene en avisos previos de Clippy en `render/src/fx.rs` e `pdf/src/import_scan.rs`; no representa una ejecución completa de todos los tests del workspace. La comprobación específica de UI también encuentra avisos previos fuera de esta paleta (Pathfinder y thumbnail worker).

Además pasaron las 12 pruebas unitarias de plugins y las 10 de su ABI. Las pruebas temporales de ABI se ejecutaron con `wasmi` optimizado (`cargo test -p vectorcraft-plugins --test abi --config 'profile.dev.package.wasmi.opt-level=3' -- --test-threads=1`). Las capturas nativas cubren sólido/imagen en 280 px claro y 340 px oscuro. Una comprobación local con Ghostscript `tiffsep` a 600 dpi sobre una imagen sintética confirmó amarillo al 100 % dentro del trap a tres alturas y la presencia de las otras planchas de la imagen; no sustituye validación de producción.

Los archivos de ejemplo SVG y PDF del usuario se comprobaron localmente, sin incorporarlos al repositorio: cuatro encuentros por archivo y fuentes sin cambios. Las pruebas sintéticas de los cuatro casos cubren ambos apilados, ancho y tinta exactos, sobreimpresión, guardado, deshacer/rehacer, exclusión y entradas no admitidas. Las separaciones del PDF se inspeccionaron además con Ghostscript `tiffsep` a 600 dpi: la franja añade la tinta sólida al 100 % en tres alturas de cada encuentro, conservando las demás tintas dentro del redondeo de 8 bits observado (hasta 0,4 puntos porcentuales). Es una comprobación técnica local, no validación en máquina de impresión.

Capturas opcionales del panel real (requieren un adaptador gráfico disponible):

```sh
TRAPCOLOR_SCREENSHOTS=/tmp/trapcolor-panel cargo test -p vectorcraft-ui-egui --test trapping_panel trapping_panel_visual_capture
```

Cubren un panel con análisis y traps en tema oscuro de 340 px y claro de 280 px. Añade `KX27_GRADIENT_CAPTURE=1` para mostrar un par sólido/degradado o `KX27_IMAGE_CAPTURE=1` para sólido/imagen. `KX27_TECHNICAL_CAPTURE=1` abre la captura experimental en lugar del análisis de contactos.

## Compilación web y GitHub Pages

El workflow **Project KX27 web** compila la aplicación completa con Trunk y deja un artefacto `trapcolor-web` en GitHub Actions. Los pushes a la rama de desarrollo solo generan el artefacto. Los pushes a `main` compilan y publican en GitHub Pages.

Para alojarla, habilita **Settings → Pages → Source: GitHub Actions** en el repositorio. Una vez que el workflow esté en la rama principal, ejecútalo con **Run workflow** y la opción **publish** activada. La acción de despliegue devuelve la URL real. La configuración inicial de Pages requiere permisos de administración y no puede activarse con el `GITHUB_TOKEN` normal del workflow.

Capturas opcionales de fase 2: `KX27_BOUNDARY_CAPTURE=1 TRAPCOLOR_SCREENSHOTS=/tmp/kx27-phase2 cargo test -p vectorcraft-ui-egui --test trapping_panel trapping_panel_visual_capture`. Añade `KX27_SOURCE_CAPTURE=1` para revisar la primera zona y mostrar sus controles de objetos en capturas de 280/340 px. `KX27_BOUNDARY_CAPTURE_SVG` permite revisar un SVG propio local sin incorporarlo a las pruebas. Para generar el diagnóstico local del motor: `KX27_BOUNDARY_SVG=/ruta/propia.svg cargo test -p vectorcraft-engine --test trapping_capture optional_real_job_boundary_capture -- --nocapture`.

Capturas de comprobación de curvas: añade `KX27_CURVE_CAPTURE=1 KX27_SOURCE_CAPTURE=1 KX27_BOUNDARY_CAPTURE=1` al comando de captura de paleta; usa una elipse cúbica sintética y muestra el resultado de verificar el primer par.

Capturas de previsualización local: añade `KX27_PREVIEW_CAPTURE=1` a los indicadores de captura de curvas. La fixture es una elipse sintética y revisa la paleta clara de 280 px y oscura de 340 px, sin usar archivos de usuarios.

Validación de previsualización local (2026-10-28): pasaron 6 pruebas nuevas del motor (ancho nominal y ambas direcciones, curvas cúbicas, oclusión a lo ancho, recortes/agujeros, spots/tint, más de 80 originales, huecos/entradas/presupuestos sin ediciones), 8 de curvas, 18 de captura/atribución, 24 del trapping existente y 17 de paleta. La paleta comprueba dirección, comparación, JSON vigente/obsoleto antes del siguiente frame y conservación de documento/selección/historial; la captura WGPU usa una elipse sintética en claro/280 px y oscuro/340 px. La revisión visual independiente y documental no detectó defectos materiales dentro del alcance temporal. Pasaron formato, assets (320), brands y layers (28), y la compilación WASM de la UI y sus dependencias modificadas. El CI completo se detiene en avisos previos de Clippy en render/fx.rs y pdf/import_scan.rs; los chequeos enfocados conservan únicamente los anteriores en Layer Panel, Pathfinder y thumbnail worker. No se afirma CI general verde, benchmark comparativo ni validación de producción.

Captura de incorporación: añade `KX27_INCORPORATE_CAPTURE=1` a los indicadores de captura de previsualización. Guarda la vista revisada y el panel después de incorporar, en 280/340 px y temas claro/oscuro, con sólidos CMYK sintéticos.

Validación de incorporación local (2026-10-29): pasaron 7 pruebas nuevas del motor para geometría exacta en ambas direcciones, tinta/tint/OP, selección e historial, guardado/reapertura nativa y PDF Separation/OP/OPM, tickets obsoletos/consumidos/reemplazados, CMS, grupos de deshacer, duplicados persistidos, reutilización/cambios de capa, restricciones de impresión y protección frente al recálculo anterior. La optimización final de comparación de coberturas se revalidó con esas 7 pruebas. También pasaron las 24 de trapping previo, 18 de captura/atribución, 8 de curvas, 6 de previsualización y 18 de paleta: **81 pruebas específicas**. La prueba de paleta confirma el botón, la geometría revisada exacta, el diagnóstico posterior con localTraps, la invalidación de mapas y Undo/Redo. Las capturas WGPU antes/después se revisaron en claro/280 px y oscuro/340 px; las revisiones visual y documental independientes no encontraron defectos materiales dentro del alcance. Pasó el gate WASM de 22 paquetes, con comprobación final de la UI y dependencias tras las guardas, además de formato, assets (320), brands y layers (28). El CI general conserva los avisos previos de Clippy en render/fx.rs y pdf/import_scan.rs; el chequeo enfocado del motor conserva el anterior de Layer Panel. No se declara CI general verde ni validación RIP/producción o rendimiento comparativo.
