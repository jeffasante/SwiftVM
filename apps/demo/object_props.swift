func main() -> Int {
    let user = allocObject("User")
    user.age = 42
    return user.age
}
