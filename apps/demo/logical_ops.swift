func main() -> Int {
    let a = 1 < 2 && 2 < 3
    let b = false || a
    if a && b {
        return 1
    }
    return 0
}
