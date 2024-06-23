use std::io;
use payments::payments::Payment;

fn main ()
{
    // 'add' adds a particpant 
    // 'part' sets a participant as a participant of a task, so they have to
    // pay as part of that task
    // 'pay' is for the people that paid for a task and need money back, they
    // are automatically added as participants of that task
    // 'print' prints out the list of participants and how much they pay
    println! ("usage:\n\
			  add NAME...\n\
			  part TASK PARTICIPANT...\n\
			  pay PARTICIPANT TASK AMOUNT\n\
			  print [-a|NAME...]\n");
    let mut pay = Payment::new ();
    loop
    {
        let mut input = String::new ();
        io::stdin ()
            .read_line (&mut input)
            .expect ("failed to read from stdin");
        let result = pay.command (&input);
        match result
        {
            Ok (()) => (),
            Err (msg) => println! ("{}", msg),
        }
        //dbg! (&pay);
    }
}
