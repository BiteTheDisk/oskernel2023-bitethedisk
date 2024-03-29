在目前的实现中信号总共有63种，1-31为非实时信号，34-63是实时信号。32和33为未定义信号。
信号是每个进程独有的，除此之外每个进程还有信号掩码。
涉及信号处理的共有3个系统调用：SYS\_SIGACTION，SYS\_SIGPROCMASK，SYS\_SIGRETURN。

## 涉及的系统调用
sys\_sigaction用于为一个信号注册信号处理函数，当进程接受到信号后会跳转到这个信号处理函数。一般信号处理函数后会调用sigreturn将程序上下文恢复到执行信号处理函数之前的状态。但程序也可能不调用sigreturn，而是使用longjump跳转到别的位置，内核不关心信号处理函数是否返回，不在内核中维护信号处理上下文信息，而是在执行信号处理函数之前将上下文压入用户栈，并在sigreturn时从用户栈中恢复信息。在执行信号处理函数的过程中，这些保存的上下文信息可能被用户程序修改，用户程序可以借此返回到不同的地方。

sys\_sigprocmask用于修改进程的信号掩码。信号掩码可以用来屏蔽信号，被屏蔽的信号会被阻塞，直到信号掩码不再阻塞该信号时该信号才会被处理。
9号信号和19号信号不能被阻塞。
sys\_sigreturn会从用户栈中取出上下文信息将程序恢复到信号处理前的状态。
sys\_sigreturn没有返回值，在执行完后不应将执行结果写入a0寄存器。

由于执行信号处理函数时与执行其他用户函数没有区别，linux的信号设计自然支持信号处理的嵌套，只要正确实现了这几个系统调用，无需在内核内保存额外信息就可以支持信号的嵌套调用。
一般信号的产生是通过sys\_kill等系统调用产生的，除此之外进程退出时也会产生信号。
信号处理的时机是不确定的。现在内核中会在每次返回用户态之前检查有无需要处理的信号。
## 与其他系统调用的交互
sys\_wait4 、sys\_read、sys\_futex等具有阻塞等待行为的系统调用可以被信号中断，如果在阻塞过程中有到来的信号应该停止等待，返回被中断的错误码。
fork 出来的子进程应该继承父进程的注册的信号处理程序，和信号掩码。
exec 后程序应该清空信号处理程序但是保留信号掩码。
