import { Config } from "@jest/types";

const conf: Config.InitialOptions = {
    modulePathIgnorePatterns: ['/src/core/', '<rootDir>/src/**/*.test.ts', 'src/core/test/tokens.test.ts'],
    moduleFileExtensions: ['js'],
    testEnvironment: 'node',
    preset: 'ts-jest'
}

export default conf;