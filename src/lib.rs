pub mod payments
{
    use std::collections::{HashMap, HashSet};
    use std::fs::File;
    use std::io::{BufWriter, BufReader};
    use regex::Regex;
    use serde::{Serialize, Deserialize};

    type PaymentResult = Result<(), String>;
    
    #[derive(Debug, Deserialize, Serialize)]
    struct Participant
    {
        pub name: String,
        pub tasks: HashSet<String>,
        pub paid_tasks: HashSet<String>,
        pub sum: Option<f32>,
    }
    
    #[derive(Debug, Deserialize, Serialize)]
    struct Task
    {
        pub name: String,
        pub owner: String,
        pub participants: HashSet<String>,
        pub cost: i32,
    }

    #[derive(Debug, Deserialize, Serialize)]
    pub struct Payment
    {
        participants: HashMap<String, Participant>,
        tasks: HashMap<String, Task>,
    }

    impl Payment
    {
        pub fn new () -> Self
        {
            Self {
                participants: HashMap::new (),
                tasks: HashMap::new ()
            }
        }

        pub fn command (&mut self, com: &str) -> PaymentResult
        {
            let parts = Regex::new (r"\s+")
                .unwrap ()
                .split (com)
                .collect::<Vec<&str>> ();
            let end = &parts[1..parts.len () - 1];
            match parts.get (0)
            {
                Some (&"add")   => return self.add (end),
                Some (&"part")  => return self.part (end),
                Some (&"pay")   => return self.pay (end),
                Some (&"print") => self.print (end),
                Some (&"save")  => return self.save (end),
                Some (&"load")  => return self.load (end),
                Some (&"rename")=> return self.rename (end),
                Some (&"remove")=> return self.remove (end),
                Some (&a)       => return Err (format! ("{} is not recognized as a command", a)),
                None            => return Err (String::from ("syntax error")),
            }
            Ok (())
        }

        fn load (&mut self, args: &[&str]) -> PaymentResult
        {
            let filename = match args.get (0)
            {
                Some (&f) => f,
                None => return Err (String::from ("Not enough arguments")),
            };
            let file = match File::open (filename)
            {
                Ok (f) => f,
                Err (_) => return Err (format! ("Unable to open file {}", filename)),
            };
            let payment: Payment = match serde_json::from_reader (BufReader::new (file))
            {
                Ok (pay) => pay,
                Err (e) => return Err (format! ("Error deserializing file:\n{}", e)),
            };
            self.participants = payment.participants;
            self.tasks = payment.tasks;
            Ok (())
        }


        fn save (&mut self, args: &[&str]) -> PaymentResult
        {
            self.calculate ();
            match args.get (0)
            {
                Some (&s) => return self.save_file (s),
                None => return self.save_string (),
            }
        }

        fn save_file (&self, filename: &str) -> PaymentResult
        {
            let file = match File::create (filename)
            {
                Ok (f) => f,
                Err (_) => return Err (format! ("Unable to open file {}", filename)),
            };
            let write = BufWriter::new (file);
            match serde_json::to_writer_pretty (write, &self)
            {
                Ok (_) => Ok (()),
                Err (e) => Err (format! ("Error serializing the object:\n{}", e)),
            }
        }
        
        fn save_string (&self) -> PaymentResult
        {
            let json = match serde_json::to_string_pretty (&self)
            {
                Ok (val) => val,
                Err (e) => return Err (format! ("Something went wrong serializing the object:\n{}", e)),
            };
            println! ("{}", json);
            Ok (())
        }

        fn rename (&mut self, args: &[&str]) -> PaymentResult
        {
            if args.len () < 2
            {
                return Err (String::from ("Not enough arguments"));
            }
            // see if we are renaming a participant
            if let Some (mut part) = self.participants.remove (args[0])
            {
                // renaming a participant
                for task_name in &part.tasks
                {
                    let task = self.tasks.get_mut (task_name).unwrap ();
                    task.participants.remove (&part.name);
                    task.participants.insert (String::from (args[1]));
                    if part.paid_tasks.contains (task_name)
                    {
                        task.owner = String::from (args[1]);
                    }
                }
                part.name = String::from (args[1]);
                self.participants.insert (String::from (args[1]), part);

                return Ok (())
            }
            // check if there is a task with this name
            if let Some (mut task) = self.tasks.remove (args[0])
            {
                for name in &task.participants
                {
                    let part = self.participants.get_mut (name).unwrap ();
                    part.tasks.remove (&task.name);
                    part.tasks.insert (String::from (args[1]));
                    if part.paid_tasks.remove (&task.name)
                    {
                        part.paid_tasks.insert (String::from (args[1]));
                    }
                }
                task.name = String::from (args[1]);
                self.tasks.insert (String::from (args[1]), task);

                return Ok (());
            }
            // nothing can be renamed, return error
            return Err (format! ("No task or participant found named {}", args[0]));
        }

        fn remove (&mut self, args: &[&str]) -> PaymentResult
        {
            if args.is_empty ()
            {
                return Err (String::from ("Not enough arguments"));
            }
            // check if the removal is a participant
            if let Some (part) = self.participants.remove (args[0])
            {
                // remove this participant from all of their tasks
                for task_name in &part.tasks
                {
                    if part.paid_tasks.contains (task_name)
                    {
                        continue;
                    }
                    let task = self.tasks.get_mut (task_name).unwrap ();
                    task.participants.remove (&part.name);
                }
                // remove all tasks this participant owns
                for task_name in &part.paid_tasks
                {
                    self.remove_task (task_name)?;
                }
                return Ok (());
            }
            match self.remove_task (args[0])
            {
                Ok (_) => Ok (()),
                Err (_) => Err (format! ("{} is not a task or participant", args[0])),
            }
            //if let Some (task) = self.tasks.remove (args[0])
            //{
            //    for name in &task.participants
            //    {
            //        let part = self.participants.get_mut (name).unwrap ();
            //        part.tasks.remove (&task.name);
            //        part.paid_tasks.remove (&task.name);
            //    }
            //    return Ok (());
            //}
        }

        fn remove_task (&mut self, task_name: &str) -> PaymentResult
        {
            let Some (task) = self.tasks.remove (task_name) else
            {
                return Err (format! ("Task {} was not present to be removed", task_name));
            };
            for name in &task.participants
            {
                let Some (part) = self.participants.get_mut (name) else
                {
                    continue;
                };
                part.tasks.remove (task_name);
                part.paid_tasks.remove (task_name);
            }
            Ok (())
        }

        fn calculate (&mut self)
        {
            for part in self.participants.values_mut ()
            {
                let mut sum = 0f32;
                for task_name in &part.tasks
                {
                    // divide the cost of this task among its participants and
                    // add that amount to the amount this participant owes
                    let task = self.tasks.get (task_name).unwrap ();
                    sum += task.cost as f32 / task.participants.len () as f32;
                }
                for task_name in &part.paid_tasks
                {
                    // same as before but subtracting since this participant
                    // has already paid their share of this task
                    let task = self.tasks.get (task_name).unwrap ();
                    sum -= task.cost as f32;
                }
                part.sum = Some (sum.round () / 100f32);
            }
        }

        fn print_participant (&self, part: &Participant)
        {
                println! ("{} owes {}", part.name, part.sum.unwrap ());
                if !part.tasks.is_empty ()
                {
                    println! ("  participated in:");
                }
                for task_name in &part.tasks
                {
                    println! ("    {task_name}: {}",
                              self.tasks.get (task_name)
                              .unwrap ()
                              .cost as f32 / 100f32);
                }
                if !part.paid_tasks.is_empty ()
                {
                    println! ("  paid for:");
                }
                for task_name in &part.paid_tasks
                {
                    println! ("    {task_name}: {}",
                              self.tasks.get (task_name)
                              .unwrap ()
                              .cost as f32 / 100f32);
                }
        }

        fn print_task (&self, task: &Task)
        {
            println! ("{} paid {} for {}", task.owner, task.cost as f32 / 100f32, task.name);
            println! ("  participants: {}", task.participants.len ());
            for part in &task.participants
            {
                println! ("    {}", part);
            }
        }

        fn print (&mut self, args: &[&str])
        {
            self.calculate ();
            let mut normal = true;
            for &arg in args
            {
                if arg == "-a"
                {
                    for part in self.participants.values ()
                    {
                        self.print_participant (&part);
                    }
                    return;
                }
                if arg == "-t"
                {
                    for task in self.tasks.values ()
                    {
                        self.print_task (&task);
                    }
                    return;
                }
                let part = match self.participants.get (arg)
                {
                    Some (val) => val,
                    None => continue,
                };
                self.print_participant (&part);
                normal = false;
            }
            if normal
            {
                for part in self.participants.values ()
                {
                    println! ("{} owes {}", part.name, part.sum.unwrap ());
                }
                return;
            }
        }

        fn add (&mut self, args: &[&str]) -> PaymentResult
        {
            if args.is_empty ()
            {
                return Err (String::from ("Not enough arguments"));
            }
            for &arg in args
            {
                let name = match arg
                {
                    "" => return Err (String::from ("Not enough arguments")),
                    "-a" => return Err (String::from ("invalid name")),
                    n => n,
                };
                // if there is already a participant with this name, we don't want
                // to overwrite them
                if self.participants.contains_key (name)
                {
                    return Err(format! ("participant {name} was already added"));
                }
                self.participants.insert (String::from (name), Participant
                                  {
                                      name: String::from (name),
                                      tasks: HashSet::new (),
                                      paid_tasks: HashSet::new (),
                                      sum: None,
                                  });
            }
            Ok (())
        }

        fn pay (&mut self, args: &[&str]) -> PaymentResult
        {
            let name = match args.get (0)
            {
                Some (&"") => return Err (String::from ("Not enough arguments")),
                Some (&n) => n,
                None => return Err (String::from ("Not enough arguments")),
            };
            let task_name = match args.get (1)
            {
                Some (&"") => return Err (String::from ("Not enough arguments")),
                Some (&n) => n,
                None => return Err (String::from ("Not enough arguments")),
            };
            let price_string = match args.get (2)
            {
                Some (&"") => return Err (String::from ("Not enough arguments")),
                Some (&n) => n,
                None => return Err (String::from ("Not enough arguments")),
            };
            let price = match price_string.parse::<f32> ()
            {
                Ok (p) => p,
                Err (_) => return Err (format! (
                        "{} not a valid decimal number for the price"
                        , price_string)),
            };
            // if this participant doesn't yet exist, add them
            if !self.participants.contains_key (name)
            {
                self.participants.insert (String::from (name), Participant
                                  {
                                      name: String::from (name),
                                      tasks: HashSet::new (),
                                      paid_tasks: HashSet::new (),
                                      sum: None,
                                  });
            }
            // see if the task is being edited or added
            let task = match self.tasks.get_mut (task_name)
            {
                Some (val) =>
                {
                    // this task already exists, check if the owner should be
                    // changed and change the cost
                    val.cost = (price * 100f32) as i32;
                    if val.owner != name
                    {
                        let owner = &val.owner;
                        let part = self.participants.get_mut (owner).unwrap ();
                        part.tasks.remove (&val.name);
                        part.paid_tasks.remove (&val.name);
                        val.participants.remove (owner);
                        val.owner = String::from (name);
                    }
                    val
                },
                None =>
                {
                    self.tasks.insert (String::from (task_name), Task
                                  {
                                      name: String::from (task_name),
                                      owner: String::from (name),
                                      participants: HashSet::new (),
                                      cost: (price * 100f32) as i32,
                                  });
                    self.tasks.get_mut (task_name).unwrap ()
                },
            };
            // add this task to the paid tasks of the participant
            let participant = self.participants.get_mut (name).unwrap ();
            participant.paid_tasks.insert (String::from (task_name));
            participant.tasks.insert (String::from (task_name));
            //self.tasks.get_mut (task_name)
            //    .unwrap ()
                task.participants.insert (String::from (name));
            Ok (())
        }

        fn part (&mut self, args: &[&str]) -> PaymentResult
        {
            if args.len () <= 1
            {
                return Err (String::from ("Not enough arguments"));
            }
            let task_name = match args.get (0)
            {
                Some (&"") => return Err (String::from ("Not enough arguments")),
                Some (&n) => n,
                None => return Err (String::from ("Not enough arguments")),
            };
            let task = match self.tasks.get_mut (task_name)
            {
                Some (result) => result,
                None => return Err (format! ("Task {task_name} has not yet been added")),
            };
            for &arg in &args[1..]
            {
                let participant = match self.participants.get_mut (arg)
                {
                    Some (result) => result,
                    None => continue,   // ignore it if they entered a bad name
                };
                // if this participant is paying for this task, don't add it to their list of tasks
                //if participant.paid_tasks.contains (task_name)
                //{
                //    return Ok (());
                //}
                // done with preparing, add stuff together
                participant.tasks.insert (String::from (task_name));
                participant.sum = None;
                task.participants.insert (String::from (arg));
            }
            Ok (())
        }
    }
}
