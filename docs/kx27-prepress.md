# KX27 Preprensa

KX27.10 · 2026-11-09 · primera versión de desarrollo. El flujo se integra en la aplicación Rust/egui existente, con el mismo motor de PDF, separaciones y trapping. No hay un segundo motor ni un servidor de archivos.

1. En el editor, elige **Window / Ventana → KX27 · Preprensa**. En una compilación web que incluya este cambio, `?modo=preprensa` entra directamente al escritorio.
2. Pulsa **Abrir PDF…** y elige el arte de producción. Se conserva el archivo original. La barra incluye mano, selección, zoom, ajuste de página y deshacer.
3. En **Tintas**, activa **Separaciones** y marca o desmarca las planchas. Alt al pulsar una casilla aísla esa plancha. Usa **Compuesta** para volver. En pantallas estrechas, **Tintas** y **Trapping** abren un panel a la vez.
4. **Mapeo de tintas** renombra una plana o la unifica con otra plana del documento. Al unificar, se conserva el nombre y el color de referencia del destino; cada uso conserva su porcentaje importado. Cada operación se puede deshacer. Se hace antes de detectar; no recalcula franjas incorporadas previamente. CMYK y la conversión de planas a proceso quedan fuera de este paso.
5. En **Trapping**, usa Detectar → Zona y par → Verificar curvas → Previsualizar → Incorporar → Diagnóstico. Reutiliza las comprobaciones, el recorte al receptor y las omisiones vigentes del flujo de desarrollo.
6. **Exportar PDF…** abre los ajustes de exportación de una copia. Por defecto, no adjunta datos de edición internos. Revisa los avisos y las separaciones de salida.

**Alcance.** Los PDF se abren y procesan en el navegador en la versión WASM. El flujo de preprensa está pensado para PDF; no valida archivos `.AI`. Usa el importador y exportador existentes: el PDF resultante se reconstruye y puede tener diferencias en archivos complejos. En particular, el exportador actual cuantiza los porcentajes de tintas planas a 8 bits; el mapeo conserva los valores que recibió del importador. Todavía no se certifica fidelidad universal para DeviceN, transparencias, imágenes o efectos complejos. El trapping local conserva sus restricciones anteriores.

**Validación.** La prueba de integración abre un PDF sintético sin datos de edición incrustados, verifica renombre y unificación de planas, sus porcentajes importados, deshacer y exportación de trapping como `/Separation` con sobreimpresión. La prueba de interfaz comprueba el escritorio y el retorno al editor sin cambiar el documento. El flujo de Actions está configurado para generar un artefacto web de la rama sin desplegarlo encima del editor publicado.
