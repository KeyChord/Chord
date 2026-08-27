enum FixtureError: Error {
    case boom(String)
}

func run(_ handlerArguments: [String], _ eventArguments: [String]) throws {
    throw FixtureError.boom("expected failure \(eventArguments)")
}
