/** Stable Panduck CLI exit codes (see Living CLI contract). */
export const ExitCode = {
    Success: 0,
    LossPolicyViolation: 1,
    InvalidArgs: 2,
    InputUnsupported: 3,
    ParseFailure: 4,
    IrOrWriterFailure: 5,
    OutputFailure: 6,
    BudgetExceeded: 7,
    BatchFailure: 8,
} as const;

export type ExitCodeValue = (typeof ExitCode)[keyof typeof ExitCode];
