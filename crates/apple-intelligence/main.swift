import Foundation
import FoundationModels

// This executable is intentionally a tiny process boundary. jst talks to it
// using JSON so the Rust CLI does not need Swift bindings or a Rust wrapper for
// Apple's beta-only FoundationModels framework.

private struct Request: Decodable {
    let systemPrompt: String
    let userPrompt: String
    let explain: Bool
}

private struct Response: Encodable {
    let command: String
    let effects: Effects
    let matchesRequest: Bool
    let explanation: String
    let parts: [Part]

    enum CodingKeys: String, CodingKey {
        case command
        case effects
        case matchesRequest = "matches_request"
        case explanation
        case parts
    }
}

private struct Effects: Encodable {
    let readsData: Bool
    let modifiesData: Bool
    let deletesData: Bool
    let usesNetwork: Bool
    let changesRemoteData: Bool
    let changesProcesses: Bool
    let installsSoftware: Bool
    let usesPrivilege: Bool
    let executesRemoteCode: Bool

    enum CodingKeys: String, CodingKey {
        case readsData = "reads_data"
        case modifiesData = "modifies_data"
        case deletesData = "deletes_data"
        case usesNetwork = "uses_network"
        case changesRemoteData = "changes_remote_data"
        case changesProcesses = "changes_processes"
        case installsSoftware = "installs_software"
        case usesPrivilege = "uses_privilege"
        case executesRemoteCode = "executes_remote_code"
    }
}

private struct Part: Encodable {
    let fragment: String
    let meaning: String
    let source: String
}

@Generable(description: "A complete, safety-described shell-command translation for JST.")
private struct GeneratedTranslation {
    @Guide(description: "One complete executable shell command. Use '# unable to translate' when no safe, compatible translation is possible.")
    var command: String

    @Guide(description: "Concrete effects of running the command.")
    var effects: GeneratedEffects

    @Guide(description: "True only when the command completely implements the request.")
    var matchesRequest: Bool

    @Guide(description: "A short standalone explanation of what the command does.")
    var explanation: String
}

@Generable(description: "A complete, safety-described shell-command translation for JST with semantic command fragments.")
private struct GeneratedDetailedTranslation {
    @Guide(description: "One complete executable shell command. Use '# unable to translate' when no safe, compatible translation is possible.")
    var command: String

    @Guide(description: "Concrete effects of running the command.")
    var effects: GeneratedEffects

    @Guide(description: "True only when the command completely implements the request.")
    var matchesRequest: Bool

    @Guide(description: "A short standalone explanation of what the command does.")
    var explanation: String

    @Guide(description: "One to eight command fragments in order. Their fragments must concatenate exactly to command.")
    var parts: [GeneratedPart]
}

@Generable(description: "Concrete effects of a shell command.")
private struct GeneratedEffects {
    var readsData: Bool
    var modifiesData: Bool
    var deletesData: Bool
    var usesNetwork: Bool
    var changesRemoteData: Bool
    var changesProcesses: Bool
    var installsSoftware: Bool
    var usesPrivilege: Bool
    var executesRemoteCode: Bool
}

@Generable(description: "A semantic fragment of the generated command.")
private struct GeneratedPart {
    var fragment: String
    var meaning: String
    var source: String
}

@main
private struct AppleIntelligenceHelper {
    static func main() async {
        do {
            if CommandLine.arguments.dropFirst().first == "--status" {
                try writeJSON(["status": availabilityStatus()])
                return
            }

            guard case .available = SystemLanguageModel.default.availability else {
                throw HelperError.unavailable(availabilityStatus())
            }

            let request = try JSONDecoder().decode(
                Request.self,
                from: FileHandle.standardInput.readDataToEndOfFile()
            )
            let session = LanguageModelSession(instructions: request.systemPrompt)
            let response: Response
            if request.explain {
                let generated = try await session.respond(
                    to: request.userPrompt,
                    generating: GeneratedDetailedTranslation.self
                ).content
                response = Response(
                    command: generated.command,
                    effects: effects(for: generated.effects),
                    matchesRequest: generated.matchesRequest,
                    explanation: generated.explanation,
                    parts: generated.parts.map {
                        Part(fragment: $0.fragment, meaning: $0.meaning, source: $0.source)
                    }
                )
            } else {
                let generated = try await session.respond(
                    to: request.userPrompt,
                    generating: GeneratedTranslation.self
                ).content
                response = Response(
                    command: generated.command,
                    effects: effects(for: generated.effects),
                    matchesRequest: generated.matchesRequest,
                    explanation: generated.explanation,
                    parts: []
                )
            }
            try writeJSON(response)
        } catch {
            FileHandle.standardError.write(
                Data(("jst-apple-intelligence: \(error.localizedDescription)\n").utf8)
            )
            exit(1)
        }
    }

    private static func availabilityStatus() -> String {
        switch SystemLanguageModel.default.availability {
        case .available:
            return "available"
        case .unavailable(let reason):
            return "unavailable: \(reason)"
        @unknown default:
            return "unavailable: unknown reason"
        }
    }

    private static func writeJSON<T: Encodable>(_ value: T) throws {
        let encoder = JSONEncoder()
        encoder.outputFormatting = [.sortedKeys]
        let data = try encoder.encode(value)
        FileHandle.standardOutput.write(data)
    }

    private static func effects(for generated: GeneratedEffects) -> Effects {
        Effects(
            readsData: generated.readsData,
            modifiesData: generated.modifiesData,
            deletesData: generated.deletesData,
            usesNetwork: generated.usesNetwork,
            changesRemoteData: generated.changesRemoteData,
            changesProcesses: generated.changesProcesses,
            installsSoftware: generated.installsSoftware,
            usesPrivilege: generated.usesPrivilege,
            executesRemoteCode: generated.executesRemoteCode
        )
    }
}

private enum HelperError: LocalizedError {
    case unavailable(String)

    var errorDescription: String? {
        switch self {
        case .unavailable(let status):
            return "Apple Intelligence is \(status)"
        }
    }
}
