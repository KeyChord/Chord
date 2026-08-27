struct MathError: Error {}

func run(_ handlerArguments: [String], _ eventArguments: [String]) throws {
    if chord_test_add(2, 3) != 5 {
        throw MathError()
    }
}
