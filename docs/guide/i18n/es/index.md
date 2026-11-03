---
title: Guía de ButtonsCLI
description: Cómo usar ButtonsCLI, una terminal nativa y rápida con botones de comandos, temas por panel, fondos con shaders en la GPU y estilos creados con IA.
machineTranslated: true
sourceHash: e856c63f314e
---

# Guía de ButtonsCLI

ButtonsCLI es una terminal nativa para quien prefiere hacer clic antes que
memorizar combinaciones de teclas. Los comandos largos van en botones, los
paneles se organizan solos y cada panel puede tener su propio tema, fuente y
fondo animado con shaders. La IA puede escribir los shaders por ti.

<!-- IMAGE: hero.jpg — three or four panes, each with a different bundle look
     and shader, command dock visible. Use a built-in bundle, not a real host. -->
![Tres terminales con fondos animados de shaders y una barra de comandos](../images/main.png)

## Empieza aquí

| Si quieres… | Lee |
| --- | --- |
| Instalarlo y abrir tus primeras terminales | [Primeros pasos](getting-started.md) |
| Organizar paneles y poner comandos en botones | [Espacio de trabajo y botones](workspace.md) |
| Cambiar colores, fuentes y efectos | [Temas](themes.md) |
| Dar a cada panel su propio estilo y guardar el conjunto | [Paquetes de temas](theme-bundles.md) |
| Añadir un fondo animado detrás del texto | [Fondos con shaders](shaders.md) |
| Que la IA escriba shaders por ti | [Shaders con IA](ai-shaders.md) |
| Que la IA cree un tema completo | [Temas con IA](ai-themes.md) |
| Conectar ChatGPT o una clave de API | [Configurar la IA](ai-setup.md) |
| Escribir o ajustar el código de un shader | [Referencia del código de shaders](shader-code.md) |
| Compartir tus estilos o incluirlos en la app | [Compartir estilos](sharing.md) |
| Ver lo que pueden hacer los estilos incluidos | [Galería de temas](theme-gallery.md) |
| Arreglar algo que se ve mal | [Solución de problemas](troubleshooting.md) |

## Las ideas detrás

- **Los paneles son posiciones.** Pane 1, Pane 2… son los lugares en la
  pantalla. Los estilos, los paquetes y "Edit for" hablan de posiciones, así
  que un paquete funciona con cualquier terminal.
- **Nada te bloquea.** Cambiar de panel o cerrar la configuración nunca te
  pide guardar antes; los cambios sin guardar se guardan como una copia
  numerada.
- **El texto sigue siendo legible.** Los shaders van *detrás* del texto. El
  control Strength define cuánto brillan, y el texto de la terminal nunca se
  distorsiona.
- **La IA es opcional.** Todo funciona sin ella. Cuando la usas, tú eliges el
  proveedor y el modelo, y las claves se guardan en el almacén de credenciales
  de tu sistema.
