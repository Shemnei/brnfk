package com.github.shemnei.brnfk

import java.io.File
import java.util.*
import kotlin.collections.ArrayList
import kotlin.system.exitProcess

enum class CommandKind {
    IncPtr,
    DecPtr,
    Inc,
    Dec,
    Output,
    Input,
    JmpStart,
    JmpEnd,
}

sealed class Command(val kind: CommandKind)
class SimpleCommand(kind: CommandKind) : Command(kind)
class JumpStartCommand(var matching: Int): Command(CommandKind.JmpStart)
class JumpEndCommand(var matching: Int): Command(CommandKind.JmpEnd)

class Tape {
    val inner: ArrayList<Byte> = ArrayList()

    private fun ensureSize(size: Int) {
        if (size >= this.inner.size) {
            for (i in this.inner.size..size) {
                this.inner.add(0)
            }
        }
    }

    fun inc(index: Int) {
        this.ensureSize(index + 1)
        ++this.inner[index]
    }

    fun dec(index: Int) {
        this.ensureSize(index + 1)
        --this.inner[index]
    }

    fun set(index: Int, value: Byte) {
        this.ensureSize(index + 1)
        this.inner[index] = value
    }

    fun get(index: Int): Byte {
        this.ensureSize(index + 1)
        return this.inner[index]
    }
}

class Program(val commands: List<Command>) {
    companion object {
        fun load(data: ByteArray): Program {
            val jumps = Stack<Int>()
            val commands = ArrayList<Command>()

            for ((i, b) in data.iterator().withIndex()) {
                val c = b.toChar()
                if (c.isWhitespace()) {
                    continue
                }

                val command = when (b.toChar()) {
                    '>' -> SimpleCommand(CommandKind.IncPtr)
                    '<' -> SimpleCommand(CommandKind.DecPtr)
                    '+' -> SimpleCommand(CommandKind.Inc)
                    '-' -> SimpleCommand(CommandKind.Dec)
                    '.' -> SimpleCommand(CommandKind.Output)
                    ',' -> SimpleCommand(CommandKind.Input)
                    '[' -> {
                        jumps.push(commands.size)
                        JumpStartCommand(0)
                    }
                    ']' -> {
                        val idx = commands.size;
                        val matchingIdx = jumps.pop();

                        when(val matching = commands[matchingIdx]) {
                            is JumpStartCommand -> matching.matching = idx
                            else -> throw IllegalStateException("matching command is not a jump start")
                        }

                        JumpEndCommand(matchingIdx)
                    }
                    else -> throw IllegalArgumentException("invalid command $b at $i")
                }

                commands.add(command)
            }

            if (jumps.isNotEmpty()) {
                throw IllegalStateException("the program contains unmatched jumps")
            }

            return Program(commands)
        }
    }
}

interface Intput: Iterator<Byte>
object StdinInput: Intput {
    override fun hasNext(): Boolean {
        return true
    }

    override fun next(): Byte {
        while (true) {
            val c = readLine().let { it?.get(0)?.toByte() }
            if (c != null) return c
        }
    }
}

interface Output {
    fun write(value: Byte)
}
object StdoutOutput: Output {
    override fun write(value: Byte) {
        print(value.toChar())
    }
}


class Brainfuck(val input: Intput = StdinInput, val output: Output = StdoutOutput) {
    fun run(program: Program) {
        val commands = program.commands
        val tape = Tape()
        var dataPtr = 0
        var instrPtr = 0

        while (instrPtr < commands.size) {
            val command = commands[instrPtr]

            when (command) {
                is SimpleCommand -> {
                    when (command.kind) {
                        CommandKind.IncPtr -> ++dataPtr
                        CommandKind.DecPtr -> --dataPtr
                        CommandKind.Inc -> tape.inc(dataPtr)
                        CommandKind.Dec -> tape.dec(dataPtr)
                        CommandKind.Output -> this.output.write(tape.get(dataPtr))
                        CommandKind.Input -> tape.set(dataPtr, this.input.next())
                        else -> throw IllegalStateException("simple command cant contain a jump command")
                    }
                }
                is JumpStartCommand -> {
                    if (tape.get(dataPtr) == 0.toByte()) {
                        instrPtr = command.matching
                        continue
                    }
                }
                is JumpEndCommand -> {
                    if (tape.get(dataPtr) != 0.toByte()) {
                        instrPtr = command.matching
                        continue
                    }
                }
            }

            ++instrPtr
        }
    }
}

/// Help text for cli usage.
const val HELP_TEXT: String = """brnfk - A brainfuck interpreter written in rust.
USAGE: brnfk [INPUT_FILE]""";

fun main(args: Array<String>) {
    if (args.size != 1) {
        print(HELP_TEXT)
        exitProcess(1)
    }

    val data = File(args[0]).readBytes()
    val program = Program.load(data)
    val brnfk = Brainfuck()
    brnfk.run(program)
}