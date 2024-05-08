# Proyecto Uwu Language

Bienvenido al repositorio oficial de Uwu Language, un lenguaje de programación diseñado para simplificar el aprendizaje de programación con una sintaxis intuitiva y fácil de comprender. Este proyecto incluye un intérprete escrito en JavaScript que permite a los usuarios ejecutar programas escritos en Uwu en cualquier entorno que soporte Node.js.

## Características

- **Sintaxis intuitiva**: Uwu Language ha sido diseñado con una sintaxis que es fácil de leer y escribir, reduciendo la curva de aprendizaje para los programadores principiantes.
- **Tipado**: Incorpora un sistema de tipado para prevenir errores comunes.

**Este lenguaje de programación no esta completo**

| Caracteristicas | ¿Esta disponible? |
|---|---|
| Analizador léxico | 🧰 |
| Pila para manejo de memoria | ✔️ |
| Error General | ✔️ |
| Errores Específicos | 🧰 |
| Funciones | ❌ |
| Clases | ❌ |
| Variables | 🧰 |
| Estructuras de control | ❌ |

## Instalación

Para instalar y comenzar a trabajar con Uwu Language, sigue estos pasos:

```bash
git clone https://github.com/beresiartejuan/uwu.git
cd uwu
npm install
npm run build
```

## Uso básico

Para ejecutar un script en Uwu, usa el siguiente comando en la terminal:

```bash
npm start
```

Aquí tienes un ejemplo simple de cómo se ve un script en Uwu:

```uwu
// Definir una variable
let numero: number = 10;
const nombre: string = "Juan";

// Definición de una función
function suma(a: number, b: number): number {
    return a + b;
};

// Uso de estructuras de control
if (numero > 5) {
    numero = suma(numero, 15);
};
```

## Estructura del Proyecto

El código fuente del proyecto se estructura de la siguiente manera:

- `src/`: Directorio con todos los archivos de código fuente.
  - `lexer.js`: Define los tokens utilizados por el lenguaje.
  - `parser.js`: Analiza los tokens para construir el árbol de sintaxis abstracta.
  - `main.js`: Punto de entrada del intérprete que ejecuta el código Uwu.
  - `example.uwu`: El script que ejecuta el intérprete.
