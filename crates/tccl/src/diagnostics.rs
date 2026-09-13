//! Explanations and suggested fixes for compile and runtime errors, in English,
//! Brazilian Portuguese and Spanish.
//!
//! Error *messages* are part of the compiler's observable behaviour (and for
//! version 1, of chain history), so they stay in English. Tools such as the CLI
//! (`tccl explain`) and the playground add a stable code, a short explanation and
//! a suggested fix in the reader's language.

use crate::error::{CompileError, VmError};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Lang {
    En,
    PtBr,
    Es,
}

impl Lang {
    pub fn parse(s: &str) -> Lang {
        match s.to_ascii_lowercase().as_str() {
            "pt" | "pt-br" | "pt_br" => Lang::PtBr,
            "es" => Lang::Es,
            _ => Lang::En,
        }
    }

    fn idx(self) -> usize {
        self as usize
    }
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
pub struct Explanation {
    pub code: &'static str,
    pub title: String,
    pub explanation: String,
    pub fix: String,
}

type Tr = [&'static str; 3];

pub struct Entry {
    pub code: &'static str,
    /// Substrings of the English message that identify the error (any matches).
    pub patterns: &'static [&'static str],
    pub title: Tr,
    pub explanation: Tr,
    pub fix: Tr,
}

/// Compile errors, most specific first.
pub const COMPILE: &[Entry] = &[
    Entry {
        code: "C001",
        patterns: &["tabs are not allowed"],
        title: ["Tab character", "Caractere de tabulação", "Carácter de tabulación"],
        explanation: [
            "Blocks are defined by indentation and TCCL only accepts spaces, so the meaning of a line never depends on editor settings.",
            "Os blocos são definidos pela indentação e a TCCL só aceita espaços, para que o significado de uma linha nunca dependa do editor.",
            "Los bloques se definen por la sangría y TCCL solo acepta espacios, para que el significado de una línea no dependa del editor.",
        ],
        fix: ["Replace tabs with 4 spaces.", "Troque as tabulações por 4 espaços.", "Reemplace las tabulaciones por 4 espacios."],
    },
    Entry {
        code: "C002",
        patterns: &["indentation does not match", "unexpected indentation", "an indented block", "empty block"],
        title: ["Indentation", "Indentação", "Sangría"],
        explanation: [
            "After a line ending with ':' the next lines must be indented more than it; a block ends when the indentation returns to an outer level.",
            "Depois de uma linha que termina com ':' as próximas linhas precisam estar mais indentadas; o bloco termina quando a indentação volta a um nível anterior.",
            "Después de una línea que termina en ':' las siguientes deben tener más sangría; el bloque termina cuando la sangría vuelve a un nivel exterior.",
        ],
        fix: [
            "Indent the body of every block with the same number of spaces (4 is customary); use 'pass' for an empty block.",
            "Indente o corpo de cada bloco com o mesmo número de espaços (normalmente 4); use 'pass' para um bloco vazio.",
            "Sangre el cuerpo de cada bloque con el mismo número de espacios (normalmente 4); use 'pass' para un bloque vacío.",
        ],
    },
    Entry {
        code: "C003",
        patterns: &["unterminated", "unknown escape"],
        title: ["Unfinished text", "Texto não terminado", "Texto sin terminar"],
        explanation: [
            "A text literal must start and end with \" on the same line. Escapes: \\n \\t \\\" \\\\.",
            "Um texto precisa começar e terminar com \" na mesma linha. Escapes: \\n \\t \\\" \\\\.",
            "Un texto debe empezar y terminar con \" en la misma línea. Escapes: \\n \\t \\\" \\\\.",
        ],
        fix: ["Close the quote, or escape inner quotes as \\\".", "Feche as aspas ou escape aspas internas como \\\".", "Cierre las comillas o escape las internas como \\\"."],
    },
    Entry {
        code: "C004",
        patterns: &["invalid number literal", "integer literal out of range", "hex bytes literal", "invalid hex literal"],
        title: ["Invalid literal", "Literal inválido", "Literal inválido"],
        explanation: [
            "Integers are decimal (underscores allowed) and fit in 128 bits; bytes are written 0x followed by an even number of hex digits. There are no decimals: amounts are integers in motes.",
            "Inteiros são decimais (com _ opcional) e cabem em 128 bits; bytes são 0x seguido de um número par de dígitos hexadecimais. Não há decimais: valores são inteiros em motes.",
            "Los enteros son decimales (con _ opcional) y caben en 128 bits; los bytes son 0x seguido de un número par de dígitos hexadecimales. No hay decimales: los montos son enteros en motes.",
        ],
        fix: ["Write amounts as integers, e.g. 5 * TCN or 250_000_000.", "Escreva valores como inteiros, ex.: 5 * TCN ou 250_000_000.", "Escriba montos como enteros, p. ej. 5 * TCN o 250_000_000."],
    },
    Entry {
        code: "C005",
        patterns: &["'contract <Name>'", "only one contract", "a contract needs at least one", "only contains modules"],
        title: ["Contract layout", "Estrutura do contrato", "Estructura del contrato"],
        explanation: [
            "A deployable file starts with 'contract Name' and declares at least one init, action or view. Modules may follow as 'module name' sections.",
            "Um arquivo publicável começa com 'contract Nome' e declara ao menos um init, action ou view. Módulos podem vir depois em seções 'module nome'.",
            "Un archivo desplegable empieza con 'contract Nombre' y declara al menos un init, action o view. Los módulos pueden seguir como secciones 'module nombre'.",
        ],
        fix: ["Put 'contract MyContract' on the first line of code.", "Coloque 'contract MeuContrato' na primeira linha de código.", "Ponga 'contract MiContrato' en la primera línea de código."],
    },
    Entry {
        code: "C006",
        patterns: &["unknown name", "unknown variable"],
        title: ["Unknown name", "Nome desconhecido", "Nombre desconocido"],
        explanation: [
            "Every name must be declared before use: a local 'let', a parameter, a const, a state variable or a built-in context value (caller, value, balance, height, self, origin).",
            "Todo nome precisa ser declarado: um 'let' local, um parâmetro, uma const, uma variável de estado ou um valor de contexto (caller, value, balance, height, self, origin).",
            "Todo nombre debe declararse: un 'let' local, un parámetro, una const, una variable de estado o un valor de contexto (caller, value, balance, height, self, origin).",
        ],
        fix: ["Check the spelling or declare it, e.g. 'let total: int = 0'.", "Confira a grafia ou declare, ex.: 'let total: int = 0'.", "Revise la ortografía o declárelo, p. ej. 'let total: int = 0'."],
    },
    Entry {
        code: "C007",
        patterns: &["unknown type"],
        title: ["Unknown type", "Tipo desconhecido", "Tipo desconocido"],
        explanation: [
            "Types are int, bool, text, bytes, address, list[T], map[K, V] (state only) and the records, enums and interfaces you declare.",
            "Os tipos são int, bool, text, bytes, address, list[T], map[K, V] (só em estado) e os records, enums e interfaces que você declarar.",
            "Los tipos son int, bool, text, bytes, address, list[T], map[K, V] (solo en estado) y los records, enums e interfaces que declare.",
        ],
        fix: ["Use one of the built-in types or declare the record/enum.", "Use um tipo embutido ou declare o record/enum.", "Use un tipo integrado o declare el record/enum."],
    },
    Entry {
        code: "C008",
        patterns: &["unknown function", "takes 0 argument", "takes 1 argument", "takes 2 argument", "takes 3 argument", "argument(s),"],
        title: ["Function call", "Chamada de função", "Llamada a función"],
        explanation: [
            "Calls must name a built-in or a 'fn' helper with the right number and types of arguments.",
            "Chamadas precisam usar uma função embutida ou um 'fn' com a quantidade e os tipos certos de argumentos.",
            "Las llamadas deben usar una función integrada o un 'fn' con la cantidad y los tipos correctos de argumentos.",
        ],
        fix: ["Check the name and the parameters in the declaration.", "Confira o nome e os parâmetros na declaração.", "Revise el nombre y los parámetros en la declaración."],
    },
    Entry {
        code: "C009",
        patterns: &["expected int, found", "expected bool, found", "expected text, found", "expected bytes, found", "expected address, found", "cannot add", "arithmetic needs", "cannot compare", "ordering comparisons", "need bool operands", "compound assignment is not defined", "expected a "],
        title: ["Type mismatch", "Tipos incompatíveis", "Tipos incompatibles"],
        explanation: [
            "TCCL never converts types implicitly: an operation or assignment needs exactly the declared type. Conditions must be bool.",
            "A TCCL nunca converte tipos sozinha: uma operação ou atribuição precisa exatamente do tipo declarado. Condições precisam ser bool.",
            "TCCL nunca convierte tipos implícitamente: una operación o asignación necesita exactamente el tipo declarado. Las condiciones deben ser bool.",
        ],
        fix: [
            "Convert explicitly with to_text, to_bytes or to_int, or change the declared type.",
            "Converta explicitamente com to_text, to_bytes ou to_int, ou mude o tipo declarado.",
            "Convierta explícitamente con to_text, to_bytes o to_int, o cambie el tipo declarado.",
        ],
    },
    Entry {
        code: "C010",
        patterns: &["must return a", "must return on every path", "does not return a value"],
        title: ["Missing return", "Falta return", "Falta return"],
        explanation: [
            "A function that declares '-> type' must end with 'return', or with an if/else whose every branch returns.",
            "Uma função que declara '-> tipo' precisa terminar com 'return', ou com um if/else em que todo ramo retorna.",
            "Una función que declara '-> tipo' debe terminar con 'return', o con un if/else donde cada rama retorna.",
        ],
        fix: ["Add a final 'return <value>'.", "Adicione um 'return <valor>' no final.", "Agregue un 'return <valor>' final."],
    },
    Entry {
        code: "C011",
        patterns: &["a view cannot", "changes state, sends TCN or emits events", "not available in a view", "view cannot call the action"],
        title: ["Views only read", "Views só leem", "Las views solo leen"],
        explanation: [
            "A view is a free query that anyone can run without a transaction, so it cannot change state, send TCN, emit events, read 'value' or call actions.",
            "Uma view é uma consulta gratuita que qualquer um roda sem transação, por isso não pode mudar estado, enviar TCN, emitir eventos, ler 'value' nem chamar actions.",
            "Una view es una consulta gratuita que cualquiera ejecuta sin transacción, así que no puede cambiar estado, enviar TCN, emitir eventos, leer 'value' ni llamar actions.",
        ],
        fix: ["Turn it into an 'action', or move the change into an action.", "Transforme em 'action' ou mova a mudança para uma action.", "Conviértala en 'action' o mueva el cambio a una action."],
    },
    Entry {
        code: "C012",
        patterns: &["already declared", "is reserved", "reserved name", "duplicate"],
        title: ["Name already used", "Nome já usado", "Nombre ya usado"],
        explanation: [
            "Each name is declared once per contract and cannot shadow built-in names (caller, value, len, ...).",
            "Cada nome é declarado uma vez por contrato e não pode esconder nomes embutidos (caller, value, len, ...).",
            "Cada nombre se declara una vez por contrato y no puede ocultar nombres integrados (caller, value, len, ...).",
        ],
        fix: ["Choose a different name.", "Escolha outro nome.", "Elija otro nombre."],
    },
    Entry {
        code: "C013",
        patterns: &["maps can only be used as state", "map keys must be", "cannot assign a whole", "is a map;", "is a state list"],
        title: ["Maps and state lists", "Maps e listas de estado", "Maps y listas de estado"],
        explanation: [
            "Maps exist only as state variables and are used item by item; state lists and maps cannot be copied or assigned as a whole.",
            "Maps só existem como variáveis de estado e são usados item a item; listas e maps de estado não podem ser copiados ou atribuídos inteiros.",
            "Los maps solo existen como variables de estado y se usan elemento por elemento; listas y maps de estado no se copian ni asignan enteros.",
        ],
        fix: ["Use m[key], m.has(key), m.remove(key), xs[i], xs.push(v), len(xs).", "Use m[chave], m.has(chave), m.remove(chave), xs[i], xs.push(v), len(xs).", "Use m[clave], m.has(clave), m.remove(clave), xs[i], xs.push(v), len(xs)."],
    },
    Entry {
        code: "C014",
        patterns: &["is a constant", "is not a constant", "constant values can only", "constant expression", "constants must be"],
        title: ["Constants", "Constantes", "Constantes"],
        explanation: [
            "Constants are computed at compile time from literals, other constants, arithmetic, enum variants and address(\"...\"), and never change.",
            "Constantes são calculadas na compilação a partir de literais, outras constantes, aritmética, variantes de enum e address(\"...\"), e nunca mudam.",
            "Las constantes se calculan al compilar a partir de literales, otras constantes, aritmética, variantes de enum y address(\"...\"), y nunca cambian.",
        ],
        fix: ["Use a state variable for values that change.", "Use uma variável de estado para valores que mudam.", "Use una variable de estado para valores que cambian."],
    },
    Entry {
        code: "C015",
        patterns: &["can be payable", "not payable in its interface", "'with value'", "views cannot receive TCN"],
        title: ["Receiving TCN", "Receber TCN", "Recibir TCN"],
        explanation: [
            "Only functions marked 'payable' (actions and init) can receive TCN; a call to another contract can attach TCN with 'with value' only if the interface marks it payable.",
            "Só funções marcadas 'payable' (actions e init) recebem TCN; uma chamada a outro contrato só pode anexar TCN com 'with value' se a interface a marcar como payable.",
            "Solo las funciones marcadas 'payable' (actions e init) reciben TCN; una llamada a otro contrato solo puede adjuntar TCN con 'with value' si la interfaz la marca payable.",
        ],
        fix: ["Add 'payable' to the header: action buy() payable:", "Adicione 'payable' ao cabeçalho: action buy() payable:", "Agregue 'payable' al encabezado: action buy() payable:"],
    },
    Entry {
        code: "C016",
        patterns: &["is an entry point"],
        title: ["Entry points are not helpers", "Entry points não são auxiliares", "Los entry points no son auxiliares"],
        explanation: [
            "init, actions and views are called by transactions and queries; code can only call 'fn' helpers (and other contracts through interfaces).",
            "init, actions e views são chamadas por transações e consultas; o código só chama auxiliares 'fn' (e outros contratos por interfaces).",
            "init, actions y views los llaman transacciones y consultas; el código solo llama auxiliares 'fn' (y otros contratos mediante interfaces).",
        ],
        fix: ["Move the shared logic into a 'fn' and call it from both places.", "Mova a lógica comum para um 'fn' e chame dos dois lugares.", "Mueva la lógica común a un 'fn' y llámelo desde ambos lugares."],
    },
    Entry {
        code: "C017",
        patterns: &["does nothing as a statement", "outside of a loop", "range() can only", "chained comparisons"],
        title: ["Statement", "Instrução", "Instrucción"],
        explanation: [
            "A line must do something: assign, call a function, control flow. Comparisons cannot be chained (use 'and').",
            "Uma linha precisa fazer algo: atribuir, chamar função, controlar fluxo. Comparações não podem ser encadeadas (use 'and').",
            "Una línea debe hacer algo: asignar, llamar una función, controlar el flujo. Las comparaciones no se encadenan (use 'and').",
        ],
        fix: ["Assign the result with 'let' or remove the line.", "Guarde o resultado com 'let' ou remova a linha.", "Guarde el resultado con 'let' o elimine la línea."],
    },
    Entry {
        code: "C018",
        patterns: &["record", "has no field", "needs every field", "named fields", "contains itself"],
        title: ["Records", "Records", "Records"],
        explanation: [
            "A record groups named, typed fields. Build it with every field named, e.g. Order(buyer: caller, amount: 5); read with order.amount.",
            "Um record agrupa campos nomeados e tipados. Construa com todos os campos nomeados, ex.: Order(buyer: caller, amount: 5); leia com order.amount.",
            "Un record agrupa campos con nombre y tipo. Constrúyalo nombrando todos los campos, p. ej. Order(buyer: caller, amount: 5); léalo con order.amount.",
        ],
        fix: ["List every field once with 'name: value'.", "Informe cada campo uma vez com 'nome: valor'.", "Indique cada campo una vez con 'nombre: valor'."],
    },
    Entry {
        code: "C019",
        patterns: &["variant", "'->'"],
        title: ["Enums and transitions", "Enums e transições", "Enums y transiciones"],
        explanation: [
            "An enum lists named situations. Lines like 'Open -> Paid, Cancelled' declare which changes are allowed; any other change fails when stored.",
            "Um enum lista situações nomeadas. Linhas como 'Open -> Paid, Cancelled' declaram as mudanças permitidas; qualquer outra falha ao gravar.",
            "Un enum enumera situaciones con nombre. Líneas como 'Open -> Paid, Cancelled' declaran los cambios permitidos; cualquier otro falla al guardar.",
        ],
        fix: ["Use Enum.Variant and list allowed next variants after '->'.", "Use Enum.Variante e liste as próximas permitidas após '->'.", "Use Enum.Variante y liste las siguientes permitidas tras '->'."],
    },
    Entry {
        code: "C020",
        patterns: &["'only", "unknown role", "is a role"],
        title: ["Roles and permissions", "Papéis e permissões", "Roles y permisos"],
        explanation: [
            "'only' restricts who may call an action: members of a role (role admin; grant admin to x) or the address in a state variable.",
            "'only' restringe quem pode chamar uma action: membros de um papel (role admin; grant admin to x) ou o endereço de uma variável de estado.",
            "'only' restringe quién puede llamar una action: miembros de un rol (role admin; grant admin to x) o la dirección de una variable de estado.",
        ],
        fix: ["Declare 'role name' and grant it in init(): grant name to caller.", "Declare 'role nome' e conceda no init(): grant nome to caller.", "Declare 'role nombre' y concédalo en init(): grant nombre to caller."],
    },
    Entry {
        code: "C021",
        patterns: &["interface"],
        title: ["Interfaces", "Interfaces", "Interfaces"],
        explanation: [
            "An interface describes actions and views of another contract. Calls use it: Token(addr).transfer(to, 5). Signatures use int, bool, text, bytes, address and lists.",
            "Uma interface descreve actions e views de outro contrato. As chamadas a usam: Token(addr).transfer(to, 5). As assinaturas usam int, bool, text, bytes, address e listas.",
            "Una interfaz describe actions y views de otro contrato. Las llamadas la usan: Token(addr).transfer(to, 5). Las firmas usan int, bool, text, bytes, address y listas.",
        ],
        fix: ["Declare the function in the interface exactly as the other contract defines it.", "Declare a função na interface exatamente como o outro contrato define.", "Declare la función en la interfaz exactamente como la define el otro contrato."],
    },
    Entry {
        code: "C022",
        patterns: &["module", "std."],
        title: ["Modules", "Módulos", "Módulos"],
        explanation: [
            "A module is reusable source code compiled into the contract ('use std.token' or a 'module name' section). It has no address; its state can only be changed by its own functions.",
            "Um módulo é código reutilizável compilado dentro do contrato ('use std.token' ou uma seção 'module nome'). Não tem endereço; seu estado só muda pelas próprias funções.",
            "Un módulo es código reutilizable compilado dentro del contrato ('use std.token' o una sección 'module nombre'). No tiene dirección; su estado solo cambia con sus propias funciones.",
        ],
        fix: ["Call the module's helpers: token.mint(to, amount).", "Chame as funções do módulo: token.mint(to, amount).", "Llame las funciones del módulo: token.mint(to, amount)."],
    },
    Entry {
        code: "C023",
        patterns: &["upgrade", "was removed", "storage slot"],
        title: ["Upgrades", "Upgrades", "Actualizaciones"],
        explanation: [
            "An upgrade keeps the storage: state variables cannot be removed or change type, records keep their fields and enums keep their variants in order.",
            "Um upgrade mantém o armazenamento: variáveis de estado não podem ser removidas nem mudar de tipo, records mantêm os campos e enums mantêm as variantes na ordem.",
            "Una actualización conserva el almacenamiento: las variables de estado no se eliminan ni cambian de tipo, los records conservan sus campos y los enums sus variantes en orden.",
        ],
        fix: ["Keep old variables (you may stop using them) and add new ones.", "Mantenha as variáveis antigas (pode deixar de usá-las) e adicione novas.", "Conserve las variables antiguas (puede dejar de usarlas) y agregue nuevas."],
    },
    Entry {
        code: "C024",
        patterns: &["too many", "too large", "too deep", "too deeply", "too long", "nested too"],
        title: ["Limit reached", "Limite atingido", "Límite alcanzado"],
        explanation: [
            "The compiler bounds source size, nesting and counts so that every node can compile any contract quickly and safely.",
            "O compilador limita tamanho, aninhamento e quantidades para que todo nó compile qualquer contrato de forma rápida e segura.",
            "El compilador limita tamaño, anidamiento y cantidades para que todo nodo compile cualquier contrato de forma rápida y segura.",
        ],
        fix: ["Split long expressions with 'let' and large contracts into modules or several contracts.", "Divida expressões longas com 'let' e contratos grandes em módulos ou vários contratos.", "Divida expresiones largas con 'let' y contratos grandes en módulos o varios contratos."],
    },
    Entry {
        code: "C025",
        patterns: &["address literal", "address prefix", "invalid address"],
        title: ["Address literal", "Endereço literal", "Dirección literal"],
        explanation: [
            "address(\"...\") takes a valid bech32m address of the network being compiled for (tc1 mainnet, tct1 testnet, tcr1 regtest).",
            "address(\"...\") recebe um endereço bech32m válido da rede para a qual se compila (tc1 mainnet, tct1 testnet, tcr1 regtest).",
            "address(\"...\") recibe una dirección bech32m válida de la red para la que se compila (tc1 mainnet, tct1 testnet, tcr1 regtest).",
        ],
        fix: ["Copy the address again and check its prefix.", "Copie o endereço de novo e confira o prefixo.", "Copie de nuevo la dirección y revise el prefijo."],
    },
    Entry {
        code: "C026",
        patterns: &["expected"],
        title: ["Syntax", "Sintaxe", "Sintaxis"],
        explanation: [
            "The line does not follow the grammar at the marked position.",
            "A linha não segue a gramática na posição marcada.",
            "La línea no sigue la gramática en la posición marcada.",
        ],
        fix: ["Compare with the language reference or an example.", "Compare com a referência da linguagem ou um exemplo.", "Compare con la referencia del lenguaje o un ejemplo."],
    },
];

fn runtime_entry(code: &str) -> Option<(Tr, Tr, Tr)> {
    Some(match code {
        "R001" => (
            ["Out of fuel", "Combustível esgotado", "Sin combustible"],
            [
                "The call used all the fuel reserved by the transaction. Every change is reverted; the fee is still paid.",
                "A chamada usou todo o combustível reservado pela transação. Tudo é revertido; a taxa continua paga.",
                "La llamada usó todo el combustible reservado. Todo se revierte; la comisión se paga igual.",
            ],
            ["Bound loops, avoid iterating lists others can grow, or reserve more fuel.", "Limite laços, evite percorrer listas que outros podem aumentar, ou reserve mais combustível.", "Limite bucles, evite recorrer listas que otros pueden agrandar, o reserve más combustible."],
        ),
        "R002" => (
            ["Requirement failed", "Requisito falhou", "Requisito fallido"],
            ["A 'require' was false; the call stops and every effect is reverted.", "Um 'require' foi falso; a chamada para e todo efeito é revertido.", "Un 'require' fue falso; la llamada se detiene y todo efecto se revierte."],
            ["Read the message: it says which condition was not met.", "Leia a mensagem: ela diz qual condição não foi atendida.", "Lea el mensaje: indica qué condición no se cumplió."],
        ),
        "R003" | "R004" => (
            ["Arithmetic error", "Erro aritmético", "Error aritmético"],
            ["Integer overflow or division by zero; TCCL arithmetic is always checked.", "Estouro de inteiro ou divisão por zero; a aritmética da TCCL é sempre verificada.", "Desbordamiento o división por cero; la aritmética de TCCL siempre se verifica."],
            ["Check inputs with require before dividing; use mul_div for a × b ÷ c.", "Valide entradas com require antes de dividir; use mul_div para a × b ÷ c.", "Valide entradas con require antes de dividir; use mul_div para a × b ÷ c."],
        ),
        "R005" => (
            ["Index out of bounds", "Índice fora dos limites", "Índice fuera de rango"],
            ["An index was negative or ≥ the length (also pop on an empty list).", "Um índice foi negativo ou ≥ tamanho (também pop em lista vazia).", "Un índice fue negativo o ≥ la longitud (también pop en lista vacía)."],
            ["require i >= 0 and i < len(xs)", "require i >= 0 and i < len(xs)", "require i >= 0 and i < len(xs)"],
        ),
        "R006" | "R022" => (
            ["Size or memory limit", "Limite de tamanho ou memória", "Límite de tamaño o memoria"],
            ["A value exceeded 64 KiB, a list 4 096 items, the events limit, or running functions held more than 16 MiB.", "Um valor passou de 64 KiB, uma lista de 4 096 itens, o limite de eventos, ou as funções em execução usaram mais de 16 MiB.", "Un valor superó 64 KiB, una lista 4 096 elementos, el límite de eventos, o las funciones en ejecución usaron más de 16 MiB."],
            ["Keep data in state maps instead of large local lists.", "Guarde dados em maps de estado em vez de listas locais grandes.", "Guarde datos en maps de estado en lugar de listas locales grandes."],
        ),
        "R007" | "R019" => (
            ["Call depth", "Profundidade de chamadas", "Profundidad de llamadas"],
            ["More than 16 nested function calls or 8 contracts on the call stack.", "Mais de 16 chamadas de função aninhadas ou 8 contratos na pilha.", "Más de 16 llamadas de función anidadas u 8 contratos en la pila."],
            ["Replace recursion with loops; flatten chains of contract calls.", "Troque recursão por laços; reduza cadeias de chamadas entre contratos.", "Cambie la recursión por bucles; reduzca cadenas de llamadas entre contratos."],
        ),
        "R008" | "R009" | "R010" => (
            ["Wrong call", "Chamada incorreta", "Llamada incorrecta"],
            ["The function does not exist, is not of the kind called (action/view), or got wrong arguments.", "A função não existe, não é do tipo chamado (action/view) ou recebeu argumentos errados.", "La función no existe, no es del tipo llamado (action/view) o recibió argumentos incorrectos."],
            ["Check the contract interface (tccl abi).", "Confira a interface do contrato (tccl abi).", "Revise la interfaz del contrato (tccl abi)."],
        ),
        "R011" => (
            ["Not payable", "Não é payable", "No es payable"],
            ["TCN was sent to a function that is not 'payable'. The value is returned by the revert.", "TCN foi enviado a uma função que não é 'payable'. O valor volta com a reversão.", "Se envió TCN a una función que no es 'payable'. El valor vuelve con la reversión."],
            ["Send no value, or mark the action payable.", "Não envie valor, ou marque a action como payable.", "No envíe valor, o marque la action como payable."],
        ),
        "R012" => (
            ["Read-only", "Somente leitura", "Solo lectura"],
            ["A view (or a contract reached from a view) tried to change something.", "Uma view (ou contrato chamado por uma view) tentou mudar algo.", "Una view (o un contrato llamado desde una view) intentó cambiar algo."],
            ["Use an action.", "Use uma action.", "Use una action."],
        ),
        "R013" | "R014" => (
            ["Payment failed", "Pagamento falhou", "Pago fallido"],
            ["send() needs a positive amount and enough contract balance.", "send() precisa de valor positivo e saldo suficiente no contrato.", "send() necesita un monto positivo y saldo suficiente del contrato."],
            ["Track what the contract owes in state and check it before sending.", "Controle no estado o que o contrato deve e confira antes de enviar.", "Registre en el estado lo que el contrato debe y verifíquelo antes de enviar."],
        ),
        "R015" | "R024" => (
            ["Cannot destroy", "Não é possível destruir", "No se puede destruir"],
            ["destroy() needs empty maps and lists, and is not allowed when another contract called this one.", "destroy() exige maps e listas vazios e não é permitido quando outro contrato chamou este.", "destroy() exige maps y listas vacíos y no se permite cuando otro contrato llamó a este."],
            ["Remove every item first, and call destroy directly from a transaction.", "Remova todos os itens antes e chame destroy direto de uma transação.", "Elimine todos los elementos antes y llame destroy directamente desde una transacción."],
        ),
        "R018" => (
            ["Re-entrant call", "Chamada reentrante", "Llamada reentrante"],
            ["A contract already running in this transaction was called again. The network always blocks this to prevent re-entrancy attacks.", "Um contrato que já estava rodando nesta transação foi chamado de novo. A rede sempre bloqueia isso para impedir ataques de reentrada.", "Se volvió a llamar un contrato que ya se ejecutaba en esta transacción. La red siempre lo bloquea para evitar ataques de reentrada."],
            ["Design flows so calls go one way (A → B), and pass data as arguments or return values.", "Desenhe fluxos em que as chamadas vão num sentido (A → B) e passe dados por argumentos ou retornos.", "Diseñe flujos donde las llamadas vayan en un sentido (A → B) y pase datos por argumentos o retornos."],
        ),
        "R020" | "R021" => (
            ["Called contract", "Contrato chamado", "Contrato llamado"],
            ["There is no contract at the address, or its function does not match the interface.", "Não há contrato no endereço, ou a função dele não corresponde à interface.", "No hay contrato en la dirección, o su función no coincide con la interfaz."],
            ["Check the address and declare the interface exactly like the target's ABI.", "Confira o endereço e declare a interface igual à ABI do destino.", "Revise la dirección y declare la interfaz igual a la ABI del destino."],
        ),
        "R023" => (
            ["Transition not allowed", "Transição não permitida", "Transición no permitida"],
            ["The enum declares which changes are allowed and this one is not listed.", "O enum declara quais mudanças são permitidas e esta não está na lista.", "El enum declara qué cambios se permiten y este no está en la lista."],
            ["Follow the declared path (e.g. Paid → Shipped → Delivered) or add the transition.", "Siga o caminho declarado (ex.: Paid → Shipped → Delivered) ou adicione a transição.", "Siga el camino declarado (p. ej. Paid → Shipped → Delivered) o agregue la transición."],
        ),
        _ => (
            ["Execution error", "Erro de execução", "Error de ejecución"],
            ["The call failed and every change it made was reverted.", "A chamada falhou e toda mudança feita foi revertida.", "La llamada falló y todo cambio se revirtió."],
            ["Read the message for details.", "Leia a mensagem para detalhes.", "Lea el mensaje para más detalles."],
        ),
    })
}

/// Explains a compile error.
pub fn explain_compile(e: &CompileError, lang: Lang) -> Explanation {
    let i = lang.idx();
    let entry = COMPILE.iter().find(|x| x.patterns.iter().any(|p| e.message.contains(p)));
    match entry {
        Some(x) => Explanation {
            code: x.code,
            title: x.title[i].to_string(),
            explanation: x.explanation[i].to_string(),
            fix: match &e.help {
                Some(h) => format!("{} ({h})", x.fix[i]),
                None => x.fix[i].to_string(),
            },
        },
        None => Explanation {
            code: "C000",
            title: ["Compile error", "Erro de compilação", "Error de compilación"][i].to_string(),
            explanation: e.message.clone(),
            fix: e.help.clone().unwrap_or_default(),
        },
    }
}

/// Explains a runtime error.
pub fn explain_runtime(e: &VmError, lang: Lang) -> Explanation {
    let (t, x, f) = runtime_entry(e.code()).expect("default entry");
    let i = lang.idx();
    Explanation { code: e.code(), title: t[i].to_string(), explanation: x[i].to_string(), fix: f[i].to_string() }
}

/// Explanation of a code (`C009`, `R018`).
pub fn explain_code(code: &str, lang: Lang) -> Option<Explanation> {
    let i = lang.idx();
    let code = code.to_ascii_uppercase();
    if let Some(x) = COMPILE.iter().find(|x| x.code == code) {
        return Some(Explanation { code: x.code, title: x.title[i].into(), explanation: x.explanation[i].into(), fix: x.fix[i].into() });
    }
    if code.starts_with('R') && code.len() == 4 && code[1..].parse::<u16>().is_ok_and(|n| (1..=25).contains(&n)) {
        let (t, x, f) = runtime_entry(&code)?;
        let stable: &'static str = RUNTIME_CODES.iter().find(|c| **c == code).copied()?;
        return Some(Explanation { code: stable, title: t[i].into(), explanation: x[i].into(), fix: f[i].into() });
    }
    None
}

const RUNTIME_CODES: [&str; 25] = [
    "R001", "R002", "R003", "R004", "R005", "R006", "R007", "R008", "R009", "R010", "R011", "R012", "R013", "R014", "R015", "R016", "R017", "R018", "R019", "R020", "R021",
    "R022", "R023", "R024", "R025",
];

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{compile, CompileOptions};

    #[test]
    fn explains_common_errors_in_three_languages() {
        let e = compile("contract A\nstate total: int\nview f() -> int:\n    return totl\n", &CompileOptions::default()).unwrap_err();
        let en = explain_compile(&e, Lang::En);
        assert_eq!(en.code, "C006");
        assert!(en.fix.contains("did you mean 'total'?"));
        assert_eq!(explain_compile(&e, Lang::PtBr).title, "Nome desconhecido");
        assert_eq!(explain_compile(&e, Lang::Es).title, "Nombre desconocido");
        let tab = compile("contract A\n\taction f():\n", &CompileOptions::default()).unwrap_err();
        assert_eq!(explain_compile(&tab, Lang::En).code, "C001");
        assert_eq!(explain_runtime(&VmError::Reentrancy("x".into()), Lang::PtBr).title, "Chamada reentrante");
        assert!(explain_code("r023", Lang::Es).is_some());
        assert!(explain_code("C999", Lang::En).is_none());
        for x in COMPILE {
            for t in x.title.iter().chain(&x.explanation).chain(&x.fix) {
                assert!(!t.is_empty());
            }
        }
    }
}
