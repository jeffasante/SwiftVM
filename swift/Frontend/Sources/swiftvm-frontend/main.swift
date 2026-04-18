import Foundation
import SwiftParser
import SwiftSyntax

enum Token: Equatable {
    case identifier(String)
    case int(Int)
    case string(String)
    case bool(Bool)
    case nilLiteral
    case symbol(String)
    case eof
}

final class Lexer {
    private let chars: [Character]
    private var i: Int = 0

    init(_ input: String) {
        self.chars = Array(input)
    }

    func tokenize() -> [Token] {
        var out: [Token] = []
        while true {
            let token = nextToken()
            out.append(token)
            if token == .eof { break }
        }
        return out
    }

    private func nextToken() -> Token {
        skipWhitespace()
        guard i < chars.count else { return .eof }

        let c = chars[i]
        if isLetter(c) || c == "_" {
            return lexIdentifier()
        }
        if isDigit(c) {
            return lexInt()
        }
        if c == "\"" {
            return lexString()
        }

        if i + 1 < chars.count {
            let pair = String(chars[i...i+1])
            if ["==", "<=", ">=", "!=", "->", "&&", "||"].contains(pair) {
                i += 2
                return .symbol(pair)
            }
        }

        i += 1
        return .symbol(String(c))
    }

    private func lexIdentifier() -> Token {
        let start = i
        while i < chars.count, isLetter(chars[i]) || isDigit(chars[i]) || chars[i] == "_" {
            i += 1
        }
        let text = String(chars[start..<i])
        switch text {
        case "true": return .bool(true)
        case "false": return .bool(false)
        case "nil": return .nilLiteral
        default: return .identifier(text)
        }
    }

    private func lexInt() -> Token {
        let start = i
        while i < chars.count, isDigit(chars[i]) { i += 1 }
        let text = String(chars[start..<i])
        return .int(Int(text) ?? 0)
    }

    private func lexString() -> Token {
        i += 1
        let start = i
        while i < chars.count, chars[i] != "\"" { i += 1 }
        let text = String(chars[start..<min(i, chars.count)])
        if i < chars.count { i += 1 }
        return .string(text)
    }

    private func skipWhitespace() {
        while i < chars.count, chars[i].isWhitespace { i += 1 }
    }

    private func isLetter(_ c: Character) -> Bool {
        c.unicodeScalars.allSatisfy { CharacterSet.letters.contains($0) }
    }

    private func isDigit(_ c: Character) -> Bool {
        c.unicodeScalars.allSatisfy { CharacterSet.decimalDigits.contains($0) }
    }
}

indirect enum Expr {
    case int(Int)
    case bool(Bool)
    case string(String)
    case nilValue
    case variable(String)
    case member(base: Expr, name: String)
    case call(name: String, args: [Expr])
    case binary(lhs: Expr, op: String, rhs: Expr)
}

indirect enum AssignTarget {
    case variable(String)
    case member(base: Expr, name: String)
}

indirect enum Stmt {
    case letVar(name: String, expr: Expr)
    case assign(target: AssignTarget, expr: Expr)
    case `return`(Expr)
    case ifElse(cond: Expr, thenBody: [Stmt], elseBody: [Stmt])
    case `while`(cond: Expr, body: [Stmt])
    case block([Stmt])
    case expr(Expr)
}

struct FunctionModel {
    let name: String
    let params: [String]
    let body: [Stmt]
}

struct StateDecl {
    let raw: String
    let name: String
}

final class Parser {
    private var tokens: [Token]
    private var i: Int = 0

    init(tokens: [Token]) {
        self.tokens = tokens
    }

    func parseStatements() -> [Stmt] {
        var out: [Stmt] = []
        while !isAtEnd() {
            if matchSymbol(";") { continue }
            if let stmt = parseStatement() {
                out.append(stmt)
            } else {
                _ = advance()
            }
        }
        return out
    }

    func parseSingleExpression() -> Expr? {
        parseExpression()
    }

    private func parseStatement() -> Stmt? {
        if matchIdentifier("let") || matchIdentifier("var") {
            guard case .identifier(let name) = advance() else { return nil }
            guard matchSymbol("=") else { return nil }
            guard let expr = parseExpression() else { return nil }
            return .letVar(name: name, expr: expr)
        }

        if matchIdentifier("return") {
            guard let expr = parseExpression() else { return .return(.nilValue) }
            return .return(expr)
        }

        if matchIdentifier("if") {
            guard let cond = parseExpression() else { return nil }
            guard let thenBody = parseBlock() else { return nil }
            var elseBody: [Stmt] = []
            if matchIdentifier("else"), let parsedElse = parseBlock() {
                elseBody = parsedElse
            }
            return .ifElse(cond: cond, thenBody: thenBody, elseBody: elseBody)
        }

        if matchIdentifier("while") {
            guard let cond = parseExpression() else { return nil }
            guard let body = parseBlock() else { return nil }
            return .while(cond: cond, body: body)
        }

        if case .identifier(let name) = peek(), case .symbol("=") = peekNext() {
            _ = advance()
            _ = advance()
            guard let expr = parseExpression() else { return nil }
            return .assign(target: .variable(name), expr: expr)
        }

        if let expr = parseExpression() {
            return .expr(expr)
        }
        return nil
    }

    private func parseBlock() -> [Stmt]? {
        guard matchSymbol("{") else { return nil }
        var body: [Stmt] = []
        while !checkSymbol("}") && !isAtEnd() {
            if let stmt = parseStatement() {
                body.append(stmt)
            } else {
                _ = advance()
            }
        }
        _ = matchSymbol("}")
        return body
    }

    private func parseExpression() -> Expr? {
        parseLogicalOr()
    }

    private func parseLogicalOr() -> Expr? {
        guard var expr = parseLogicalAnd() else { return nil }
        while let op = matchAnySymbol(["||"]) {
            guard let rhs = parseLogicalAnd() else { break }
            expr = .binary(lhs: expr, op: op, rhs: rhs)
        }
        return expr
    }

    private func parseLogicalAnd() -> Expr? {
        guard var expr = parseEquality() else { return nil }
        while let op = matchAnySymbol(["&&"]) {
            guard let rhs = parseEquality() else { break }
            expr = .binary(lhs: expr, op: op, rhs: rhs)
        }
        return expr
    }

    private func parseEquality() -> Expr? {
        guard var expr = parseComparison() else { return nil }
        while let op = matchAnySymbol(["==", "!="]) {
            guard let rhs = parseComparison() else { break }
            expr = .binary(lhs: expr, op: op, rhs: rhs)
        }
        return expr
    }

    private func parseComparison() -> Expr? {
        guard var expr = parseTerm() else { return nil }
        while let op = matchAnySymbol(["<", ">", "<=", ">="]) {
            guard let rhs = parseTerm() else { break }
            expr = .binary(lhs: expr, op: op, rhs: rhs)
        }
        return expr
    }

    private func parseTerm() -> Expr? {
        guard var expr = parseFactor() else { return nil }
        while let op = matchAnySymbol(["+", "-"]) {
            guard let rhs = parseFactor() else { break }
            expr = .binary(lhs: expr, op: op, rhs: rhs)
        }
        return expr
    }

    private func parseFactor() -> Expr? {
        guard var expr = parsePrimary() else { return nil }
        while let op = matchAnySymbol(["*", "/"]) {
            guard let rhs = parsePrimary() else { break }
            expr = .binary(lhs: expr, op: op, rhs: rhs)
        }
        return expr
    }

    private func parsePrimary() -> Expr? {
        switch peek() {
        case .int(let v):
            _ = advance()
            return .int(v)
        case .bool(let v):
            _ = advance()
            return .bool(v)
        case .string(let v):
            _ = advance()
            return .string(v)
        case .nilLiteral:
            _ = advance()
            return .nilValue
        case .identifier(let name):
            _ = advance()
            if matchSymbol("(") {
                var args: [Expr] = []
                if !checkSymbol(")") {
                    while true {
                        // optional argument label: label:
                        if case .identifier = peek(), case .symbol(":") = peekNext() {
                            _ = advance()
                            _ = advance()
                        }
                        if let arg = parseExpression() { args.append(arg) }
                        if !matchSymbol(",") { break }
                    }
                }
                _ = matchSymbol(")")
                return .call(name: name, args: args)
            }
            return .variable(name)
        case .symbol("("):
            _ = advance()
            let expr = parseExpression()
            _ = matchSymbol(")")
            return expr
        default:
            return nil
        }
    }

    private func peek() -> Token {
        i < tokens.count ? tokens[i] : .eof
    }

    private func peekNext() -> Token {
        (i + 1) < tokens.count ? tokens[i + 1] : .eof
    }

    @discardableResult
    private func advance() -> Token {
        let token = peek()
        if i < tokens.count { i += 1 }
        return token
    }

    private func isAtEnd() -> Bool {
        if case .eof = peek() { return true }
        return false
    }

    private func matchIdentifier(_ name: String) -> Bool {
        if case .identifier(let current) = peek(), current == name {
            _ = advance()
            return true
        }
        return false
    }

    private func matchSymbol(_ symbol: String) -> Bool {
        if case .symbol(let current) = peek(), current == symbol {
            _ = advance()
            return true
        }
        return false
    }

    private func matchAnySymbol(_ symbols: [String]) -> String? {
        if case .symbol(let current) = peek(), symbols.contains(current) {
            _ = advance()
            return current
        }
        return nil
    }

    private func checkSymbol(_ symbol: String) -> Bool {
        if case .symbol(let current) = peek(), current == symbol {
            return true
        }
        return false
    }
}

func extractStateDecls(_ source: String) -> [StateDecl] {
    var states: [StateDecl] = source
        .split(separator: "\n", omittingEmptySubsequences: false)
        .map(String.init)
        .compactMap { raw in
            let line = raw.trimmingCharacters(in: .whitespaces)
            guard line.hasPrefix("// state ") else { return nil }
            let declaration = String(line.dropFirst("// ".count))
            let noPrefix = declaration.dropFirst("state ".count)
            let namePart = noPrefix.split(separator: ":", maxSplits: 1).first.map(String.init) ?? ""
            return StateDecl(raw: declaration, name: namePart.trimmingCharacters(in: .whitespaces))
        }

    let tree = SwiftParser.Parser.parse(source: source)
    for item in tree.statements {
        guard let varDecl = item.item.as(VariableDeclSyntax.self) else { continue }
        guard let binding = varDecl.bindings.first else { continue }
        guard let identPattern = binding.pattern.as(IdentifierPatternSyntax.self) else { continue }

        let name = identPattern.identifier.text.trimmingCharacters(in: .whitespacesAndNewlines)
        if name.isEmpty { continue }
        if states.contains(where: { $0.name == name }) { continue }

        let typeName = binding.typeAnnotation?.type.description
            .trimmingCharacters(in: .whitespacesAndNewlines) ?? "Any"

        var declaration = "state \(name):\(typeName)"
        if let initExpr = binding.initializer?.value {
            let value = initExpr.description.trimmingCharacters(in: .whitespacesAndNewlines)
            if isSupportedStateLiteral(value) {
                declaration += "=\(value)"
            }
        }

        states.append(StateDecl(raw: declaration, name: name))
    }

    return states
}

func isSupportedStateLiteral(_ raw: String) -> Bool {
    if raw == "nil" || raw == "true" || raw == "false" { return true }
    if Int(raw) != nil { return true }
    if raw.hasPrefix("\"") && raw.hasSuffix("\"") { return true }
    return false
}

func parseFunctions(_ source: String) -> [FunctionModel] {
    let tree = SwiftParser.Parser.parse(source: source)
    var functions: [FunctionModel] = []

    for item in tree.statements {
        guard let fn = item.item.as(FunctionDeclSyntax.self) else { continue }
        let name = fn.name.text.trimmingCharacters(in: .whitespacesAndNewlines)
        let params = fn.signature.parameterClause.parameters.compactMap { param -> String? in
            let first = param.firstName.text.trimmingCharacters(in: .whitespacesAndNewlines)
            if first == "_" {
                return param.secondName?.text.trimmingCharacters(in: .whitespacesAndNewlines)
            }
            return first
        }

        let statements: [Stmt]
        if let body = fn.body {
            statements = lowerCodeBlockItems(body.statements)
        } else {
            statements = []
        }
        functions.append(FunctionModel(name: name, params: params, body: statements))
    }

    return functions
}

func lowerCodeBlockItems(_ items: CodeBlockItemListSyntax) -> [Stmt] {
    var out: [Stmt] = []
    for item in items {
        if let lowered = lowerCodeBlockItem(item) {
            out.append(lowered)
        } else {
            // Fallback path keeps progress for unsupported syntax while AST lowering expands.
            out.append(contentsOf: parseStatementsFromText(item.item.description))
        }
    }
    return out
}

func lowerCodeBlockItem(_ item: CodeBlockItemSyntax) -> Stmt? {
    if let varDecl = item.item.as(VariableDeclSyntax.self) {
        return lowerVariableDecl(varDecl)
    }

    if let returnStmt = item.item.as(ReturnStmtSyntax.self) {
        if let expr = returnStmt.expression, let loweredExpr = lowerExprSyntax(expr) {
            return .return(loweredExpr)
        }
        return .return(.nilValue)
    }

    if let whileStmt = item.item.as(WhileStmtSyntax.self) {
        return lowerWhileStmt(whileStmt)
    }

    if let forStmt = item.item.as(ForStmtSyntax.self) {
        return lowerForStmt(forStmt)
    }

    if let ifExpr = item.item.as(IfExprSyntax.self) {
        return lowerIfExpr(ifExpr)
    }

    if let exprStmt = item.item.as(ExpressionStmtSyntax.self) {
        return lowerExpressionStmt(exprStmt)
    }

    if let rawExpr = item.item.as(ExprSyntax.self) {
        return lowerStandaloneExpr(rawExpr)
    }

    return nil
}

func lowerVariableDecl(_ varDecl: VariableDeclSyntax) -> Stmt? {
    guard let binding = varDecl.bindings.first else { return nil }
    guard let ident = binding.pattern.as(IdentifierPatternSyntax.self) else { return nil }
    guard let initExpr = binding.initializer?.value else { return nil }
    guard let loweredExpr = lowerExprSyntax(initExpr) else { return nil }
    return .letVar(name: ident.identifier.text, expr: loweredExpr)
}

func lowerWhileStmt(_ whileStmt: WhileStmtSyntax) -> Stmt? {
    guard let first = whileStmt.conditions.first else { return nil }
    guard let condExpr = first.condition.as(ExprSyntax.self) else { return nil }
    guard let loweredCond = lowerExprSyntax(condExpr) else { return nil }
    let body = lowerCodeBlockItems(whileStmt.body.statements)
    return .while(cond: loweredCond, body: body)
}

func lowerForStmt(_ forStmt: ForStmtSyntax) -> Stmt? {
    guard let ident = forStmt.pattern.as(IdentifierPatternSyntax.self) else { return nil }
    let loopVar = ident.identifier.text
    guard let (startRaw, op, endRaw) = extractRangeExpr(forStmt.sequence) else { return nil }
    guard let startExpr = lowerExprSyntax(startRaw) else { return nil }
    guard let endExpr = lowerExprSyntax(endRaw) else { return nil }

    let conditionOp = (op == "..<") ? "<" : "<="
    let cond = Expr.binary(lhs: .variable(loopVar), op: conditionOp, rhs: endExpr)

    var body = lowerCodeBlockItems(forStmt.body.statements)
    body.append(.assign(target: .variable(loopVar), expr: .binary(lhs: .variable(loopVar), op: "+", rhs: .int(1))))

    return .block([
        .letVar(name: loopVar, expr: startExpr),
        .while(cond: cond, body: body),
    ])
}

func extractRangeExpr(_ expr: ExprSyntax) -> (ExprSyntax, String, ExprSyntax)? {
    if let infix = expr.as(InfixOperatorExprSyntax.self) {
        let op = infix.operator.description.trimmingCharacters(in: .whitespacesAndNewlines)
        if op == "..<" || op == "..." {
            return (infix.leftOperand, op, infix.rightOperand)
        }
    }

    if let seq = expr.as(SequenceExprSyntax.self), seq.elements.count >= 3 {
        guard let lhs = seq.elements.first, let rhs = seq.elements.last else { return nil }
        let middle = seq.elements.dropFirst().dropLast()
        for element in middle {
            if let opExpr = element.as(BinaryOperatorExprSyntax.self) {
                let op = opExpr.operator.text.trimmingCharacters(in: .whitespacesAndNewlines)
                if op == "..<" || op == "..." {
                    return (lhs, op, rhs)
                }
            }
        }
    }

    return nil
}

func lowerIfExpr(_ ifExpr: IfExprSyntax) -> Stmt? {
    guard let first = ifExpr.conditions.first else { return nil }
    guard let condExpr = first.condition.as(ExprSyntax.self) else { return nil }
    guard let loweredCond = lowerExprSyntax(condExpr) else { return nil }

    let thenBody = lowerCodeBlockItems(ifExpr.body.statements)
    var elseBody: [Stmt] = []
    if let elseBodySyntax = ifExpr.elseBody {
        if let elseCodeBlock = elseBodySyntax.as(CodeBlockSyntax.self) {
            elseBody = lowerCodeBlockItems(elseCodeBlock.statements)
        } else if let elseIf = elseBodySyntax.as(IfExprSyntax.self), let nested = lowerIfExpr(elseIf) {
            elseBody = [nested]
        } else {
            elseBody = parseStatementsFromText(elseBodySyntax.description)
        }
    }

    return .ifElse(cond: loweredCond, thenBody: thenBody, elseBody: elseBody)
}

func lowerExpressionStmt(_ exprStmt: ExpressionStmtSyntax) -> Stmt? {
    lowerStandaloneExpr(exprStmt.expression)
}

func lowerStandaloneExpr(_ expr: ExprSyntax) -> Stmt? {
    if let ifExpr = expr.as(IfExprSyntax.self) {
        return lowerIfExpr(ifExpr)
    }

    if let (target, rhs) = extractAssignment(expr) {
        return .assign(target: target, expr: rhs)
    }

    guard let lowered = lowerExprSyntax(expr) else { return nil }
    return .expr(lowered)
}

func extractAssignment(_ expr: ExprSyntax) -> (AssignTarget, Expr)? {
    if let infix = expr.as(InfixOperatorExprSyntax.self),
       infix.operator.description.trimmingCharacters(in: .whitespacesAndNewlines) == "=",
       let target = extractAssignTarget(infix.leftOperand),
       let rhs = lowerExprSyntax(infix.rightOperand) {
        return (target, rhs)
    }

    if let seq = expr.as(SequenceExprSyntax.self), seq.elements.count >= 3 {
        let elements = Array(seq.elements)
        guard let assignIndex = elements.firstIndex(where: { element in
            if element.is(AssignmentExprSyntax.self) { return true }
            if let op = element.as(BinaryOperatorExprSyntax.self) {
                return op.operator.text.trimmingCharacters(in: .whitespacesAndNewlines) == "="
            }
            return false
        }) else {
            return nil
        }

        guard assignIndex > 0, assignIndex + 1 < elements.count else { return nil }
        guard let target = extractAssignTarget(elements[assignIndex - 1]) else { return nil }

        let rhsElements = elements[(assignIndex + 1)...]
        if rhsElements.count == 1, let rhs = lowerExprSyntax(rhsElements[rhsElements.startIndex]) {
            return (target, rhs)
        }

        let rhsText = rhsElements.map(\.description).joined(separator: " ")
        if let rhs = parseSingleExpressionFromText(rhsText) {
            return (target, rhs)
        }
    }

    return nil
}

func extractAssignTarget(_ lhs: ExprSyntax) -> AssignTarget? {
    if let lhsRef = lhs.as(DeclReferenceExprSyntax.self) {
        return .variable(lhsRef.baseName.text)
    }
    if let member = lhs.as(MemberAccessExprSyntax.self),
       let baseExpr = member.base,
       let loweredBase = lowerExprSyntax(baseExpr) {
        return .member(base: loweredBase, name: member.declName.baseName.text)
    }
    return nil
}

func lowerExprSyntax(_ expr: ExprSyntax) -> Expr? {
    if let prefix = expr.as(PrefixOperatorExprSyntax.self) {
        let op = prefix.operator.text.trimmingCharacters(in: .whitespacesAndNewlines)
        guard let rhs = lowerExprSyntax(prefix.expression) else { return nil }
        switch op {
        case "-":
            return .binary(lhs: .int(0), op: "-", rhs: rhs)
        case "!":
            return .binary(lhs: rhs, op: "==", rhs: .bool(false))
        default:
            break
        }
    }

    if let intExpr = expr.as(IntegerLiteralExprSyntax.self) {
        return .int(Int(intExpr.literal.text) ?? 0)
    }

    if let boolExpr = expr.as(BooleanLiteralExprSyntax.self) {
        return .bool(boolExpr.literal.text == "true")
    }

    if expr.is(NilLiteralExprSyntax.self) {
        return .nilValue
    }

    if let strExpr = expr.as(StringLiteralExprSyntax.self) {
        let pieces = strExpr.segments.compactMap { segment -> String? in
            if let seg = segment.as(StringSegmentSyntax.self) {
                return seg.content.text
            }
            return nil
        }
        return .string(pieces.joined())
    }

    if let refExpr = expr.as(DeclReferenceExprSyntax.self) {
        return .variable(refExpr.baseName.text)
    }

    if let memberExpr = expr.as(MemberAccessExprSyntax.self) {
        if let base = memberExpr.base, let loweredBase = lowerExprSyntax(base) {
            return .member(base: loweredBase, name: memberExpr.declName.baseName.text)
        }
        return .variable(memberExpr.declName.baseName.text)
    }

    if let callExpr = expr.as(FunctionCallExprSyntax.self) {
        let name: String
        if let ref = callExpr.calledExpression.as(DeclReferenceExprSyntax.self) {
            name = ref.baseName.text
        } else if let member = callExpr.calledExpression.as(MemberAccessExprSyntax.self) {
            name = member.declName.baseName.text
        } else {
            name = callExpr.calledExpression.description.trimmingCharacters(in: .whitespacesAndNewlines)
        }
        let args = callExpr.arguments.compactMap { arg in
            lowerExprSyntax(arg.expression)
        }
        return .call(name: name, args: args)
    }

    if let infix = expr.as(InfixOperatorExprSyntax.self),
       let lhs = lowerExprSyntax(infix.leftOperand),
       let rhs = lowerExprSyntax(infix.rightOperand) {
        let op = infix.operator.description.trimmingCharacters(in: .whitespacesAndNewlines)
        return .binary(lhs: lhs, op: op, rhs: rhs)
    }

    if let paren = expr.as(TupleExprSyntax.self), paren.elements.count == 1 {
        return lowerExprSyntax(paren.elements.first!.expression)
    }

    return parseSingleExpressionFromText(expr.description)
}

func parseStatementsFromText(_ text: String) -> [Stmt] {
    let lexer = Lexer(text)
    let parser = Parser(tokens: lexer.tokenize())
    return parser.parseStatements()
}

func parseSingleExpressionFromText(_ text: String) -> Expr? {
    let lexer = Lexer(text)
    let parser = Parser(tokens: lexer.tokenize())
    return parser.parseSingleExpression()
}

enum OutInstr {
    case line(String)
    case label(String)
    case jump(String)
    case jumpIfFalse(String)
}

final class Lowerer {
    private var labelCounter = 0
    private var stateNames: Set<String> = []

    func lower(functions: [FunctionModel], states: [StateDecl]) -> String {
        var output: [String] = []
        self.stateNames = Set(states.map { $0.name })

        for state in states {
            output.append(state.raw)
        }
        if !states.isEmpty { output.append("") }

        for fn in functions {
            output.append("func \(fn.name)(\(fn.params.joined(separator: ",")))")

            var locals = Set(fn.params)
            let lowered = lowerStatements(fn.body, locals: &locals)
            let concrete = resolveLabels(lowered)
            output.append(contentsOf: concrete.map { "  \($0)" })

            if concrete.last != "return" {
                output.append("  load_const nil")
                output.append("  return")
            }
            output.append("end")
            output.append("")
        }

        if functions.first(where: { $0.name == "main" }) == nil {
            output.append("func main()")
            output.append("  load_const nil")
            output.append("  return")
            output.append("end")
        }

        return output.joined(separator: "\n")
    }

    private func lowerStatements(_ statements: [Stmt], locals: inout Set<String>) -> [OutInstr] {
        var out: [OutInstr] = []
        for stmt in statements {
            switch stmt {
            case .letVar(let name, let expr):
                out.append(contentsOf: lowerExpr(expr, locals: locals))
                if stateNames.contains(name) {
                    out.append(.line("store_global \(name)"))
                } else {
                    locals.insert(name)
                    out.append(.line("store_var \(name)"))
                }
            case .assign(let target, let expr):
                switch target {
                case .variable(let name):
                    out.append(contentsOf: lowerExpr(expr, locals: locals))
                    if stateNames.contains(name) {
                        out.append(.line("store_global \(name)"))
                    } else {
                        locals.insert(name)
                        out.append(.line("store_var \(name)"))
                    }
                case .member(let base, let name):
                    out.append(contentsOf: lowerExpr(base, locals: locals))
                    out.append(contentsOf: lowerExpr(expr, locals: locals))
                    out.append(.line("set_prop \(name)"))
                    out.append(.line("pop"))
                }
            case .return(let expr):
                out.append(contentsOf: lowerExpr(expr, locals: locals))
                out.append(.line("return"))
            case .expr(let expr):
                out.append(contentsOf: lowerExpr(expr, locals: locals))
                out.append(.line("pop"))
            case .ifElse(let cond, let thenBody, let elseBody):
                let elseLabel = makeLabel("else")
                let endLabel = makeLabel("ifend")
                out.append(contentsOf: lowerExpr(cond, locals: locals))
                out.append(.jumpIfFalse(elseLabel))
                var thenLocals = locals
                out.append(contentsOf: lowerStatements(thenBody, locals: &thenLocals))
                out.append(.jump(endLabel))
                out.append(.label(elseLabel))
                var elseLocals = locals
                out.append(contentsOf: lowerStatements(elseBody, locals: &elseLocals))
                out.append(.label(endLabel))
            case .while(let cond, let body):
                let startLabel = makeLabel("while_start")
                let endLabel = makeLabel("while_end")
                out.append(.label(startLabel))
                out.append(contentsOf: lowerExpr(cond, locals: locals))
                out.append(.jumpIfFalse(endLabel))
                var bodyLocals = locals
                out.append(contentsOf: lowerStatements(body, locals: &bodyLocals))
                out.append(.jump(startLabel))
                out.append(.label(endLabel))
            case .block(let stmts):
                var nestedLocals = locals
                out.append(contentsOf: lowerStatements(stmts, locals: &nestedLocals))
            }
        }
        return out
    }

    private func lowerExpr(_ expr: Expr, locals: Set<String>) -> [OutInstr] {
        switch expr {
        case .int(let v):
            return [.line("load_const \(v)")]
        case .bool(let v):
            return [.line("load_const \(v ? "true" : "false")")]
        case .string(let v):
            let escaped = v.replacingOccurrences(of: "\"", with: "\\\"")
            return [.line("load_const \"\(escaped)\"")]
        case .nilValue:
            return [.line("load_const nil")]
        case .variable(let name):
            if stateNames.contains(name) {
                return [.line("load_global \(name)")]
            }
            if locals.contains(name) {
                return [.line("load_var \(name)")]
            }
            return [.line("load_var \(name)")]
        case .member(let base, let name):
            var out: [OutInstr] = []
            out.append(contentsOf: lowerExpr(base, locals: locals))
            out.append(.line("get_prop \(name)"))
            return out
        case .call(let name, let args):
            var out: [OutInstr] = []
            if name == "nativeCall",
               let first = args.first,
               case .string(let selector) = first {
                let nativeArgs = Array(args.dropFirst())
                for arg in nativeArgs {
                    out.append(contentsOf: lowerExpr(arg, locals: locals))
                }
                out.append(.line("native_call \(selector) \(nativeArgs.count)"))
                return out
            }
            if name == "allocObject",
               let first = args.first,
               case .string(let typeName) = first {
                out.append(.line("alloc_object \(typeName)"))
                return out
            }
            for arg in args {
                out.append(contentsOf: lowerExpr(arg, locals: locals))
            }
            if name == "print" {
                for _ in args {
                    out.append(.line("print"))
                }
                out.append(.line("load_const nil"))
                return out
            }
            out.append(.line("call \(name) \(args.count)"))
            return out
        case .binary(let lhs, let op, let rhs):
            var out: [OutInstr] = []
            out.append(contentsOf: lowerExpr(lhs, locals: locals))
            out.append(contentsOf: lowerExpr(rhs, locals: locals))
            switch op {
            case "+": out.append(.line("add"))
            case "-": out.append(.line("sub"))
            case "*": out.append(.line("mul"))
            case "/": out.append(.line("div"))
            case "==": out.append(.line("eq"))
            case "!=": out.append(.line("ne"))
            case "<": out.append(.line("lt"))
            case ">": out.append(.line("gt"))
            case "<=": out.append(.line("le"))
            case ">=": out.append(.line("ge"))
            case "&&": out.append(.line("and"))
            case "||": out.append(.line("or"))
            default: out.append(.line("add"))
            }
            return out
        }
    }

    private func resolveLabels(_ instructions: [OutInstr]) -> [String] {
        var labelToIndex: [String: Int] = [:]
        var index = 0
        for instr in instructions {
            switch instr {
            case .label(let name):
                labelToIndex[name] = index
            default:
                index += 1
            }
        }

        var out: [String] = []
        for instr in instructions {
            switch instr {
            case .line(let text):
                out.append(text)
            case .jump(let label):
                out.append("jump \(labelToIndex[label] ?? 0)")
            case .jumpIfFalse(let label):
                out.append("jump_if_false \(labelToIndex[label] ?? 0)")
            case .label:
                break
            }
        }
        return out
    }

    private func makeLabel(_ prefix: String) -> String {
        defer { labelCounter += 1 }
        return "__\(prefix)_\(labelCounter)"
    }
}

// MARK: - SwiftUI View Config Extraction

struct ViewConfigEntry {
    let key: String
    let value: String
}

final class ViewConfigExtractor: SyntaxVisitor {
    let fileName: String
    var entries: [ViewConfigEntry] = []
    private var currentStruct: String?

    init(fileName: String) {
        self.fileName = fileName
        super.init(viewMode: .sourceAccurate)
    }

    override func visit(_ node: StructDeclSyntax) -> SyntaxVisitorContinueKind {
        currentStruct = node.name.text.trimmingCharacters(in: .whitespacesAndNewlines)
        return .visitChildren
    }

    override func visitPost(_ node: StructDeclSyntax) {
        currentStruct = nil
    }

    override func visit(_ node: FunctionCallExprSyntax) -> SyntaxVisitorContinueKind {
        // Handle: Text("..."), Label("...", ...)
        if let identExpr = node.calledExpression.as(DeclReferenceExprSyntax.self) {
            let name = identExpr.baseName.text
            if ["Text", "Label"].contains(name) {
                if let firstArg = node.arguments.first,
                   let stringLit = firstArg.expression.as(StringLiteralExprSyntax.self) {
                    let value = stringLit.segments.description
                    let prefix = currentStruct.map { "\($0)." } ?? ""
                    entries.append(ViewConfigEntry(
                        key: "\(prefix)\(name).\(value)",
                        value: value
                    ))
                }
            }
        }

        // Handle: .navigationTitle("..."), .font(...), etc.
        if let memberAccess = node.calledExpression.as(MemberAccessExprSyntax.self) {
            let modifier = memberAccess.declName.baseName.text
            let trackedModifiers = [
                "navigationTitle", "navigationSubtitle",
                "tabItem", "badge",
                "accessibilityLabel", "accessibilityHint",
                "confirmationDialog", "alert",
                "headerProminence"
            ]
            if trackedModifiers.contains(modifier) {
                if let firstArg = node.arguments.first,
                   let stringLit = firstArg.expression.as(StringLiteralExprSyntax.self) {
                    let value = stringLit.segments.description
                    let prefix = currentStruct.map { "\($0)." } ?? ""
                    entries.append(ViewConfigEntry(
                        key: "\(prefix)\(modifier)",
                        value: value
                    ))
                }
            }
        }

        return .visitChildren
    }
}

func extractViewConfig(source: String, fileName: String) -> [ViewConfigEntry] {
    let tree = SwiftParser.Parser.parse(source: source)
    let extractor = ViewConfigExtractor(fileName: fileName)
    extractor.walk(tree)
    return extractor.entries
}

func viewConfigToJSON(_ entries: [ViewConfigEntry]) -> String {
    var dict: [String: String] = [:]
    for entry in entries {
        dict[entry.key] = entry.value
    }
    // Simple JSON serialization without Foundation's JSONSerialization
    let pairs = dict.sorted(by: { $0.key < $1.key }).map { key, value in
        let escapedKey = key.replacingOccurrences(of: "\"", with: "\\\"")
        let escapedVal = value.replacingOccurrences(of: "\"", with: "\\\"")
        return "  \"\(escapedKey)\": \"\(escapedVal)\""
    }
    return "{\n\(pairs.joined(separator: ",\n"))\n}"
}

// MARK: - Entry Point

let args = CommandLine.arguments
if args.count < 2 {
    fputs("usage: swiftvm-frontend [--view-config] <swift-source-file>\n", stderr)
    exit(2)
}

let viewConfigMode = args.contains("--view-config")
let sourcePath = args.first(where: { !$0.starts(with: "-") && $0 != args[0] })!
let source: String

do {
    source = try String(contentsOfFile: sourcePath)
} catch {
    fputs("failed to read source: \(error)\n", stderr)
    exit(1)
}

if viewConfigMode {
    let fileName = (sourcePath as NSString).lastPathComponent
        .replacingOccurrences(of: ".swift", with: "")
    let entries = extractViewConfig(source: source, fileName: fileName)
    print(viewConfigToJSON(entries))
} else {
    let states = extractStateDecls(source)
    let functions = parseFunctions(source)
    let lowerer = Lowerer()
    let svm = lowerer.lower(functions: functions, states: states)
    print(svm)
}
