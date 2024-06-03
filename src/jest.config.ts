import { Config } from "@jest/types";

const conf: Config.InitialOptions = {
    moduleFileExtensions: ['js'],
    testEnvironment: 'node',
    preset: 'ts-jest'
}

export default conf;