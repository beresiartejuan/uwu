import { Config } from "@jest/types";

const conf: Config.InitialOptions = {
    modulePathIgnorePatterns: ['/src/core', '<rootDir>/src/**/*.test.ts'],
    moduleFileExtensions: ['test.js'],
    testEnvironment: 'node',
    preset: 'ts-jest'
}

export default conf;